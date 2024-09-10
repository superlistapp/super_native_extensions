use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    ptr::NonNull,
    rc::{Rc, Weak},
};

use block2::RcBlock;
use irondash_engine_context::EngineContext;
use irondash_message_channel::{Late, Value};
use irondash_run_loop::{platform::PollSession, RunLoop};
use objc2::{
    ffi::NSInteger,
    rc::Id,
    runtime::{AnyObject, Bool, ProtocolObject, Sel},
    sel, ClassType,
};
use objc2_app_kit::{
    NSDragOperation, NSDraggingInfo, NSDraggingItem, NSDraggingItemEnumerationOptions,
    NSFilePromiseReceiver, NSPasteboardItem, NSView,
};
use objc2_foundation::{ns_string, NSArray, NSDictionary, NSMutableArray, NSRect, NSString};

use crate::{
    api_model::DropOperation,
    drop_manager::{
        BaseDropEvent, DropEvent, DropItem, DropSessionId, ItemPreviewRequest,
        PlatformDropContextDelegate, PlatformDropContextId,
    },
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    platform_impl::platform::os::util::{flip_rect, ns_image_from_image_data},
    reader_manager::RegisteredDataReader,
    value_promise::PromiseResult,
};

use super::{drag_common::DropOperationExt, util::class_builder_from_name, PlatformDataReader};

pub struct PlatformDropContext {
    id: PlatformDropContextId,
    weak_self: Late<Weak<Self>>,
    view: Id<NSView>,
    delegate: Weak<dyn PlatformDropContextDelegate>,
    sessions: RefCell<HashMap<isize /* draggingSequenceNumber */, Rc<Session>>>,
}

static ONCE: std::sync::Once = std::sync::Once::new();

struct Session {
    context_id: PlatformDropContextId,
    context_delegate: Weak<dyn PlatformDropContextDelegate>,
    context_view: Id<NSView>,
    id: DropSessionId,
    last_operation: Cell<DropOperation>,
    reader: Rc<PlatformDataReader>,
    registered_reader: RegisteredDataReader,
}

thread_local! {
    pub static VIEW_TO_CONTEXT: RefCell<HashMap<Id<NSView>, Weak<PlatformDropContext>>> = RefCell::new(HashMap::new());
}

impl Session {
    fn context_delegate(&self) -> NativeExtensionsResult<Rc<dyn PlatformDropContextDelegate>> {
        self.context_delegate
            .upgrade()
            .ok_or_else(|| NativeExtensionsError::OtherError("missing context delegate".into()))
    }

    fn event_from_dragging_info(
        &self,
        dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
        accepted_operation: Option<DropOperation>,
    ) -> NativeExtensionsResult<DropEvent> {
        let delegate = self.context_delegate()?;

        let dragging_sequence_number = unsafe { dragging_info.draggingSequenceNumber() };
        let drag_contexts = delegate.get_platform_drag_contexts();
        let local_data = drag_contexts
            .iter()
            .map(|c| c.get_local_data(dragging_sequence_number))
            .find(|c| c.is_some())
            .flatten()
            .unwrap_or_default();

        let location = unsafe { dragging_info.draggingLocation() }; // window coordinates
        let location = self.context_view.convertPoint_fromView(location, None);

        let operation_mask = unsafe { dragging_info.draggingSourceOperationMask() };

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
        dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
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

    fn dragging_exited(
        &self,
        _dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
    ) -> NativeExtensionsResult<()> {
        self.context_delegate()?.send_drop_leave(
            self.context_id,
            BaseDropEvent {
                session_id: self.id,
            },
        );
        Ok(())
    }

    fn enumerate_items<F>(&self, dragging_info: &ProtocolObject<dyn NSDraggingInfo>, f: F)
    where
        F: Fn(NonNull<NSDraggingItem>, NSInteger, NonNull<Bool>) + 'static,
    {
        let block = RcBlock::new(f);
        unsafe {
            let class =
                Id::retain(NSPasteboardItem::class() as *const _ as *mut AnyObject).unwrap();

            dragging_info
                .enumerateDraggingItemsWithOptions_forView_classes_searchOptions_usingBlock(
                    NSDraggingItemEnumerationOptions(0),
                    Some(&self.context_view),
                    &NSArray::from_vec(vec![class]),
                    &NSDictionary::dictionary(),
                    &block,
                );
        }
    }

    fn prepare_for_drag_operation(
        self: &Rc<Self>,
        dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
    ) -> NativeExtensionsResult<bool> {
        let delegate = self.context_delegate()?;
        let event = self.event_from_dragging_info(dragging_info, None)?;
        let animates = Rc::new(Cell::new(Bool::NO));

        let self_cloned = self.clone();
        let animates_cloned = animates.clone();
        self.enumerate_items(dragging_info, move |dragging_item, index, _| {
            let dragging_item = unsafe { Id::retain(dragging_item.as_ptr()) }.unwrap();
            let item = &event.items.get(index as usize);
            if let Some(item) = item {
                let dragging_frame = unsafe { dragging_item.draggingFrame() };
                let preview_promise = delegate.get_preview_for_item(
                    self_cloned.context_id,
                    ItemPreviewRequest {
                        session_id: self_cloned.id,
                        item_id: item.item_id,
                        size: dragging_frame.size.into(),
                        fade_out_delay: 0.330,  // 20 frames at 60fps
                        fade_out_duration: 0.0, // no animation
                    },
                );
                let mut poll_session = PollSession::new();
                let preview = loop {
                    if let Some(result) = preview_promise.try_take() {
                        match result {
                            PromiseResult::Ok { value } => break value.preview,
                            PromiseResult::Cancelled => break None,
                        }
                    }
                    RunLoop::current()
                        .platform_run_loop
                        .poll_once(&mut poll_session);
                };
                if let Some(preview) = preview {
                    animates_cloned.set(Bool::YES);
                    let mut rect: NSRect = preview.destination_rect.into();
                    flip_rect(&self_cloned.context_view, &mut rect);
                    match preview.destination_image {
                        Some(image) => {
                            let snapshot = ns_image_from_image_data(vec![image]);
                            unsafe {
                                dragging_item.setDraggingFrame_contents(rect, Some(&snapshot))
                            };
                        }
                        None => unsafe {
                            dragging_item.setDraggingFrame(rect);
                        },
                    }
                }
            }
        });
        unsafe {
            dragging_info.setAnimatesToDestination(animates.get().into());
        }
        Ok(true)
    }

    fn perform_drag_operation(
        &self,
        dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
    ) -> NativeExtensionsResult<bool> {
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
        let mut poll_session = PollSession::new();
        while !done.get() {
            RunLoop::current()
                .platform_run_loop
                .poll_once(&mut poll_session);
        }
        Ok(true)
    }

    fn dragging_ended(
        &self,
        _dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
    ) -> NativeExtensionsResult<()> {
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
    pub fn new(
        id: PlatformDropContextId,
        engine_handle: i64,
        delegate: Weak<dyn PlatformDropContextDelegate>,
    ) -> NativeExtensionsResult<Self> {
        ONCE.call_once(prepare_flutter);
        let view = EngineContext::get()?.get_flutter_view(engine_handle)?;
        Ok(Self {
            id,
            weak_self: Late::new(),
            view: unsafe { Id::cast(view) },
            delegate,
            sessions: RefCell::new(HashMap::new()),
        })
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        VIEW_TO_CONTEXT.with(|v| {
            v.borrow_mut().insert(self.view.clone(), weak_self.clone());
        });
        self.weak_self.set(weak_self);
    }

    pub fn register_drop_formats(&self, types: &[String]) -> NativeExtensionsResult<()> {
        let types: Vec<_> = types.iter().map(|ty| NSString::from_str(ty)).collect();
        let our_types = NSArray::from_vec(types);
        let promise_receiver_types = unsafe { NSFilePromiseReceiver::readableDraggedTypes() };
        let mut all_types = unsafe { NSMutableArray::<NSString>::array() };

        unsafe {
            all_types.addObjectsFromArray(&our_types);
            all_types.addObjectsFromArray(&promise_receiver_types);
            all_types.addObject(ns_string!("dev.nativeshell.placeholder-item"));
            self.view.registerForDraggedTypes(&all_types);
        }

        Ok(())
    }

    fn session_for_dragging_info(
        &self,
        dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
    ) -> NativeExtensionsResult<Rc<Session>> {
        let dragging_sequence_number = unsafe { dragging_info.draggingSequenceNumber() };

        let delegate = self
            .delegate
            .upgrade()
            .ok_or_else(|| NativeExtensionsError::OtherError("missing context delegate".into()))?;

        Ok(self
            .sessions
            .borrow_mut()
            .entry(dragging_sequence_number)
            .or_insert_with(|| {
                let pasteboard = unsafe { dragging_info.draggingPasteboard() };
                let platform_reader = PlatformDataReader::from_pasteboard(pasteboard);
                let registered_reader =
                    delegate.register_platform_reader(self.id, platform_reader.clone());
                Rc::new(Session {
                    context_id: self.id,
                    context_delegate: self.delegate.clone(),
                    context_view: self.view.clone(),
                    id: dragging_sequence_number.into(),
                    last_operation: Cell::new(DropOperation::None),
                    reader: platform_reader,
                    registered_reader,
                })
            })
            .clone())
    }

    fn dragging_updated(
        &self,
        dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
    ) -> NativeExtensionsResult<NSDragOperation> {
        self.session_for_dragging_info(dragging_info)?
            .dragging_updated(dragging_info)
    }

    fn dragging_exited(
        &self,
        dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
    ) -> NativeExtensionsResult<()> {
        self.session_for_dragging_info(dragging_info)?
            .dragging_exited(dragging_info)
    }

    fn prepare_for_drag_operation(
        &self,
        dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
    ) -> NativeExtensionsResult<bool> {
        self.session_for_dragging_info(dragging_info)?
            .prepare_for_drag_operation(dragging_info)
    }

    fn perform_drag_operation(
        &self,
        dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
    ) -> NativeExtensionsResult<bool> {
        self.session_for_dragging_info(dragging_info)?
            .perform_drag_operation(dragging_info)
    }

    fn dragging_ended(
        &self,
        dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
    ) -> NativeExtensionsResult<()> {
        let dragging_sequence_number = unsafe { dragging_info.draggingSequenceNumber() };

        let session = self.sessions.borrow_mut().remove(&dragging_sequence_number);
        match session {
            Some(session) => session.dragging_ended(dragging_info),
            None => Ok(()),
        }
    }
}

impl Drop for PlatformDropContext {
    fn drop(&mut self) {
        VIEW_TO_CONTEXT
            .try_with(|v| {
                v.borrow_mut().remove(&*self.view);
            })
            .ok();
    }
}

fn with_state<F, FR, R>(this: &NSView, callback: F, default: FR) -> R
where
    F: FnOnce(Rc<PlatformDropContext>) -> R,
    FR: FnOnce() -> R,
{
    let this = this.retain();
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
        let mut class = class_builder_from_name("FlutterView");

        class.add_method(
            sel!(draggingEntered:),
            dragging_updated as extern "C" fn(_, _, _) -> _,
        );

        class.add_method(
            sel!(draggingUpdated:),
            dragging_updated as extern "C" fn(_, _, _) -> _,
        );

        class.add_method(
            sel!(draggingExited:),
            dragging_exited as extern "C" fn(_, _, _),
        );

        class.add_method(
            sel!(prepareForDragOperation:),
            prepare_for_drag_operation as extern "C" fn(_, _, _) -> _,
        );

        class.add_method(
            sel!(performDragOperation:),
            perform_drag_operation as extern "C" fn(_, _, _) -> _,
        );

        class.add_method(
            sel!(draggingEnded:),
            dragging_ended as extern "C" fn(_, _, _) -> _,
        );

        class.add_method(
            sel!(wantsPeriodicDraggingUpdates),
            wants_periodical_dragging_updates as extern "C" fn(_, _) -> _,
        );
    }
}

extern "C" fn dragging_updated(
    this: &NSView,
    _: Sel,
    dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
) -> NSDragOperation {
    with_state(
        this,
        |state| {
            state
                .dragging_updated(dragging_info)
                .ok_log()
                .unwrap_or(NSDragOperation::None)
        },
        || NSDragOperation::None,
    )
}

extern "C" fn dragging_exited(
    this: &NSView,
    _: Sel,
    dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
) {
    with_state(
        this,
        |state| state.dragging_exited(dragging_info).ok_log().unwrap_or(()),
        || (),
    )
}

extern "C" fn prepare_for_drag_operation(
    this: &NSView,
    _: Sel,
    dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
) -> Bool {
    with_state(
        this,
        |state| {
            if state
                .prepare_for_drag_operation(dragging_info)
                .ok_log()
                .unwrap_or(false)
            {
                Bool::YES
            } else {
                Bool::NO
            }
        },
        || Bool::NO,
    )
}

extern "C" fn perform_drag_operation(
    this: &NSView,
    _: Sel,
    dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
) -> Bool {
    with_state(
        this,
        |state| {
            if state
                .perform_drag_operation(dragging_info)
                .ok_log()
                .unwrap_or(false)
            {
                Bool::YES
            } else {
                Bool::NO
            }
        },
        || Bool::NO,
    )
}

extern "C" fn dragging_ended(
    this: &NSView,
    _: Sel,
    dragging_info: &ProtocolObject<dyn NSDraggingInfo>,
) {
    with_state(
        this,
        |state| state.dragging_ended(dragging_info).ok_log().unwrap_or(()),
        || (),
    )
}

extern "C" fn wants_periodical_dragging_updates(_this: &NSView, _: Sel) -> Bool {
    Bool::NO // consistent with other platforms
}
