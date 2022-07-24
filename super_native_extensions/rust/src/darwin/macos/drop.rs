use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
};

use block::ConcreteBlock;
use cocoa::{
    appkit::NSView,
    base::{id, nil, BOOL, NO, YES},
    foundation::{NSArray, NSInteger, NSPoint, NSRect, NSUInteger},
};

use nativeshell_core::{util::Late, Context, Value};
use objc::{
    class, msg_send,
    rc::{autoreleasepool, StrongPtr},
    runtime::{Object, Sel},
    sel, sel_impl,
};

use crate::{
    api_model::DropOperation,
    drop_manager::{
        BaseDropEvent, DropEvent, DropItem, DropSessionId, ItemPreviewRequest,
        PlatformDropContextDelegate,
    },
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    platform_impl::platform::{
        common::to_nsstring,
        os::util::{flip_rect, ns_image_from_image_data},
    },
    reader_manager::RegisteredDataReader,
    value_promise::PromiseResult,
};

use super::{
    drag_common::{DropOperationExt, NSDragOperation, NSDragOperationNone},
    util::class_decl_from_name,
    PlatformDataReader,
};

pub struct PlatformDropContext {
    id: i64,
    weak_self: Late<Weak<Self>>,
    view: StrongPtr,
    delegate: Weak<dyn PlatformDropContextDelegate>,
    sessions: RefCell<HashMap<NSInteger /* draggingSequenceNumber */, Rc<Session>>>,
}

static ONCE: std::sync::Once = std::sync::Once::new();

struct Session {
    context_id: i64,
    context_delegate: Weak<dyn PlatformDropContextDelegate>,
    context_view: StrongPtr,
    id: DropSessionId,
    last_operation: Cell<DropOperation>,
    reader: Rc<PlatformDataReader>,
    registered_reader: RegisteredDataReader,
}

thread_local! {
    pub static VIEW_TO_CONTEXT: RefCell<HashMap<id, Weak<PlatformDropContext>>> = RefCell::new(HashMap::new());
}

impl Session {
    fn context_delegate(&self) -> NativeExtensionsResult<Rc<dyn PlatformDropContextDelegate>> {
        self.context_delegate
            .upgrade()
            .ok_or_else(|| NativeExtensionsError::OtherError("missing context delegate".into()))
    }

    fn event_from_dragging_info(
        &self,
        dragging_info: id,
        accepted_operation: Option<DropOperation>,
    ) -> NativeExtensionsResult<DropEvent> {
        let delegate = self.context_delegate()?;

        let dragging_sequence_number: NSInteger =
            unsafe { msg_send![dragging_info, draggingSequenceNumber] };
        let drag_context = delegate.get_platform_drag_context(self.context_id)?;
        let local_data = drag_context.get_local_data(dragging_sequence_number);

        let location: NSPoint = unsafe { msg_send![dragging_info, draggingLocation] }; // window coordinates
        let location: NSPoint =
            unsafe { NSView::convertPoint_fromView_(*self.context_view, location, nil) };

        let operation_mask: NSDragOperation =
            unsafe { msg_send![dragging_info, draggingSourceOperationMask] };

        let mut items = Vec::new();
        for (index, item) in self.reader.get_items_sync()?.iter().enumerate() {
            items.push(DropItem {
                item_id: (*item).into(),
                formats: self.reader.get_formats_for_item_sync(*item)?,
                local_data: local_data.get(index).cloned().unwrap_or(Value::Null),
            })
        }

        Ok(DropEvent {
            session_id: self.id,
            location_in_view: location.into(),
            allowed_operations: DropOperation::from_platform_mask(operation_mask),
            accepted_operation,
            items,
            reader: Some(self.registered_reader.clone()),
        })
    }

    fn dragging_updated(
        self: &Rc<Self>,
        dragging_info: id,
    ) -> NativeExtensionsResult<NSDragOperation> {
        let delegate = self.context_delegate()?;

        let event = self.event_from_dragging_info(dragging_info, None)?;
        let session_clone = self.clone();
        delegate.send_drop_update(
            self.context_id,
            event,
            Box::new(move |res| {
                let res = res.ok_log().unwrap_or(DropOperation::None);
                session_clone.last_operation.set(res);
            }),
        );

        Ok(self.last_operation.get().to_platform())
    }

    fn dragging_exited(&self, _dragging_info: id) -> NativeExtensionsResult<()> {
        self.context_delegate()?.send_drop_leave(
            self.context_id,
            BaseDropEvent {
                session_id: self.id,
            },
        );
        Ok(())
    }

    fn enumerate_items<F>(&self, dragging_info: id, f: F)
    where
        F: Fn(id, NSInteger, *mut BOOL),
    {
        let block = ConcreteBlock::new(f);
        unsafe {
            let () = msg_send![dragging_info,
                enumerateDraggingItemsWithOptions: 0 as NSUInteger
                forView: *self.context_view
                classes: NSArray::arrayWithObjects(nil, &[class!(NSPasteboardItem) as * const _ as id])
                searchOptions: nil
                usingBlock: &*block
            ];
        }
    }

    fn prepare_for_drag_operation(&self, dragging_info: id) -> NativeExtensionsResult<bool> {
        let delegate = self.context_delegate()?;
        let event = self.event_from_dragging_info(dragging_info, None)?;
        let animates = Cell::new(NO);
        self.enumerate_items(dragging_info, |dragging_item, index, _| {
            let item = &event.items.get(index as usize);
            if let Some(item) = item {
                let dragging_frame: NSRect = unsafe { msg_send![dragging_item, draggingFrame] };
                let preview_promise = delegate.get_preview_for_item(
                    self.context_id,
                    ItemPreviewRequest {
                        session_id: self.id,
                        item_id: item.item_id,
                        size: dragging_frame.size.into(),
                        fade_out_delay: 0.330,  // 20 frames at 60fps
                        fade_out_duration: 0.0, // no animation
                    },
                );
                let preview = loop {
                    if let Some(result) = preview_promise.try_take() {
                        match result {
                            PromiseResult::Ok { value } => break value.preview,
                            PromiseResult::Cancelled => break None,
                        }
                    }
                    Context::get().run_loop().platform_run_loop.poll_once();
                };
                if let Some(preview) = preview {
                    animates.set(YES);
                    let mut rect: NSRect = preview.destination_rect.into();
                    unsafe { flip_rect(*self.context_view, &mut rect) };
                    match preview.destination_image {
                        Some(image) => {
                            let snapshot = ns_image_from_image_data(vec![image]);
                            let () = unsafe {
                                msg_send![dragging_item, setDraggingFrame:rect contents:*snapshot]
                            };
                        }
                        None => {
                            let () = unsafe { msg_send![dragging_item, setDraggingFrame: rect] };
                        }
                    }
                }
            }
        });
        unsafe {
            let () = msg_send![dragging_info, setAnimatesToDestination: animates.get()];
        }
        Ok(true)
    }

    fn perform_drag_operation(&self, dragging_info: id) -> NativeExtensionsResult<bool> {
        let delegate = self.context_delegate()?;
        let event =
            self.event_from_dragging_info(dragging_info, Some(self.last_operation.get()))?;
        let done = Rc::new(Cell::new(false));
        let done_clone = done.clone();
        delegate.send_perform_drop(
            self.context_id,
            event,
            Box::new(move |r| {
                r.ok_log();
                done_clone.set(true);
            }),
        );
        while !done.get() {
            Context::get().run_loop().platform_run_loop.poll_once();
        }
        Ok(true)
    }

    fn dragging_ended(&self, _dragging_info: id) -> NativeExtensionsResult<()> {
        self.context_delegate()?.send_drop_ended(
            self.context_id,
            BaseDropEvent {
                session_id: self.id,
            },
        );
        Ok(())
    }
}

impl PlatformDropContext {
    pub fn new(id: i64, view_handle: i64, delegate: Weak<dyn PlatformDropContextDelegate>) -> Self {
        ONCE.call_once(prepare_flutter);
        Self {
            id,
            weak_self: Late::new(),
            view: unsafe { StrongPtr::retain(view_handle as *mut _) },
            delegate,
            sessions: RefCell::new(HashMap::new()),
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        VIEW_TO_CONTEXT.with(|v| {
            v.borrow_mut().insert(*self.view, weak_self.clone());
        });
        self.weak_self.set(weak_self);
    }

    pub fn register_drop_types(&self, types: &[String]) -> NativeExtensionsResult<()> {
        autoreleasepool(|| unsafe {
            let types: Vec<id> = types
                .iter()
                .map(|ty| to_nsstring(&ty).autorelease())
                .collect();
            let types = NSArray::arrayWithObjects(nil, &types);
            let _: id = msg_send![*self.view, registerForDraggedTypes: types];
        });
        Ok(())
    }

    fn session_for_dragging_info(&self, dragging_info: id) -> NativeExtensionsResult<Rc<Session>> {
        let dragging_sequence_number: NSInteger =
            unsafe { msg_send![dragging_info, draggingSequenceNumber] };

        let delegate = self
            .delegate
            .upgrade()
            .ok_or_else(|| NativeExtensionsError::OtherError("missing context delegate".into()))?;

        Ok(self
            .sessions
            .borrow_mut()
            .entry(dragging_sequence_number)
            .or_insert_with(|| {
                let pasteboard: id = unsafe { msg_send![dragging_info, draggingPasteboard] };
                let platform_reader =
                    PlatformDataReader::from_pasteboard(unsafe { StrongPtr::retain(pasteboard) });
                let registered_reader =
                    delegate.register_platform_reader(self.id, platform_reader.clone());
                Rc::new(Session {
                    context_id: self.id,
                    context_delegate: self.delegate.clone(),
                    context_view: self.view.clone(),
                    id: (dragging_sequence_number as i64).into(),
                    last_operation: Cell::new(DropOperation::None),
                    reader: platform_reader,
                    registered_reader,
                })
            })
            .clone())
    }

    fn dragging_updated(&self, dragging_info: id) -> NativeExtensionsResult<NSDragOperation> {
        self.session_for_dragging_info(dragging_info)?
            .dragging_updated(dragging_info)
    }

    fn dragging_exited(&self, dragging_info: id) -> NativeExtensionsResult<()> {
        self.session_for_dragging_info(dragging_info)?
            .dragging_exited(dragging_info)
    }

    fn prepare_for_drag_operation(&self, dragging_info: id) -> NativeExtensionsResult<bool> {
        self.session_for_dragging_info(dragging_info)?
            .prepare_for_drag_operation(dragging_info)
    }

    fn perform_drag_operation(&self, dragging_info: id) -> NativeExtensionsResult<bool> {
        self.session_for_dragging_info(dragging_info)?
            .perform_drag_operation(dragging_info)
    }

    fn dragging_ended(&self, dragging_info: id) -> NativeExtensionsResult<()> {
        let dragging_sequence_number: NSInteger =
            unsafe { msg_send![dragging_info, draggingSequenceNumber] };

        let session = self.sessions.borrow_mut().remove(&dragging_sequence_number);
        match session {
            Some(session) => session.dragging_ended(dragging_info),
            None => Ok(()),
        }
    }
}

impl Drop for PlatformDropContext {
    fn drop(&mut self) {
        VIEW_TO_CONTEXT.with(|v| {
            v.borrow_mut().remove(&*self.view);
        });
    }
}

fn with_state<F, FR, R>(this: id, callback: F, default: FR) -> R
where
    F: FnOnce(Rc<PlatformDropContext>) -> R,
    FR: FnOnce() -> R,
{
    let state = VIEW_TO_CONTEXT
        .with(|v| v.borrow().get(&this).cloned())
        .and_then(|a| a.upgrade());
    if let Some(state) = state {
        callback(state)
    } else {
        default()
    }
}

fn prepare_flutter() {
    unsafe {
        let mut class = class_decl_from_name("FlutterView");

        class.add_method(
            sel!(draggingEntered:),
            dragging_updated as extern "C" fn(&mut Object, Sel, id) -> NSDragOperation,
        );

        class.add_method(
            sel!(draggingUpdated:),
            dragging_updated as extern "C" fn(&mut Object, Sel, id) -> NSDragOperation,
        );

        class.add_method(
            sel!(draggingExited:),
            dragging_exited as extern "C" fn(&mut Object, Sel, id),
        );

        class.add_method(
            sel!(prepareForDragOperation:),
            perpare_for_drag_operation as extern "C" fn(&mut Object, Sel, id) -> BOOL,
        );

        class.add_method(
            sel!(performDragOperation:),
            perform_drag_operation as extern "C" fn(&mut Object, Sel, id) -> BOOL,
        );

        class.add_method(
            sel!(draggingEnded:),
            dragging_ended as extern "C" fn(&mut Object, Sel, id),
        );

        class.add_method(
            sel!(wantsPeriodicDraggingUpdates),
            wants_periodical_dragging_updates as extern "C" fn(&mut Object, Sel) -> BOOL,
        );
    }
}

extern "C" fn dragging_updated(this: &mut Object, _: Sel, dragging_info: id) -> NSDragOperation {
    with_state(
        this,
        |state| {
            state
                .dragging_updated(dragging_info)
                .ok_log()
                .unwrap_or(NSDragOperationNone)
        },
        || NSDragOperationNone,
    )
}

extern "C" fn dragging_exited(this: &mut Object, _: Sel, dragging_info: id) {
    with_state(
        this,
        |state| state.dragging_exited(dragging_info).ok_log().unwrap_or(()),
        || (),
    )
}

extern "C" fn perpare_for_drag_operation(this: &mut Object, _: Sel, dragging_info: id) -> BOOL {
    with_state(
        this,
        |state| {
            if state
                .prepare_for_drag_operation(dragging_info)
                .ok_log()
                .unwrap_or(NO)
            {
                YES
            } else {
                NO
            }
        },
        || NO,
    )
}

extern "C" fn perform_drag_operation(this: &mut Object, _: Sel, dragging_info: id) -> BOOL {
    with_state(
        this,
        |state| {
            if state
                .perform_drag_operation(dragging_info)
                .ok_log()
                .unwrap_or(NO)
            {
                YES
            } else {
                NO
            }
        },
        || NO,
    )
}

extern "C" fn dragging_ended(this: &mut Object, _: Sel, dragging_info: id) {
    with_state(
        this,
        |state| state.dragging_ended(dragging_info).ok_log().unwrap_or(()),
        || (),
    )
}

extern "C" fn wants_periodical_dragging_updates(_this: &mut Object, _: Sel) -> BOOL {
    NO // consistent with other platforms
}
