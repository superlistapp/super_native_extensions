use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    mem::ManuallyDrop,
    os::raw::c_void,
    rc::{Rc, Weak},
};

use block::ConcreteBlock;
use cocoa::{
    base::{id, nil, BOOL, NO, YES},
    foundation::{NSArray, NSUInteger},
};
use core_graphics::geometry::{CGPoint, CGRect};

use nativeshell_core::{platform::run_loop::PollSession, util::Late, Context, Value};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    rc::{autoreleasepool, StrongPtr},
    runtime::{Class, Object, Protocol, Sel},
    sel, sel_impl,
};
use once_cell::sync::Lazy;

use crate::{
    api_model::{DropOperation, Size},
    drop_manager::{
        BaseDropEvent, DropEvent, DropItem, DropSessionId, ItemPreview, ItemPreviewRequest,
        PlatformDropContextDelegate,
    },
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    platform_impl::platform::common::{from_nsstring, superclass, CGAffineTransformMakeScale},
    value_promise::PromiseResult, ENGINE_CONTEXT,
};

use super::{drag_common::DropOperationExt, util::image_view_from_data, PlatformDataReader};

pub struct PlatformDropContext {
    id: i64,
    weak_self: Late<Weak<Self>>,
    view: StrongPtr,
    delegate: Weak<dyn PlatformDropContextDelegate>,
    interaction: Late<StrongPtr>,
    interaction_delegate: Late<StrongPtr>,
    sessions: RefCell<HashMap<id, Rc<Session>>>,
}

struct Session {
    context_id: i64,
    context_delegate: Weak<dyn PlatformDropContextDelegate>,
    context_view: StrongPtr,
    platform_session: id,
    last_operation: Cell<DropOperation>,
    view_containers: RefCell<Vec<StrongPtr>>,
}

impl Drop for Session {
    fn drop(&mut self) {
        let containers = self.view_containers.borrow();
        for container in containers.iter() {
            let () = unsafe { msg_send![**container, removeFromSuperview] };
        }
    }
}

impl Session {
    fn context_delegate(&self) -> NativeExtensionsResult<Rc<dyn PlatformDropContextDelegate>> {
        self.context_delegate
            .upgrade()
            .ok_or_else(|| NativeExtensionsError::OtherError("missing context delegate".into()))
    }

    fn session_id(&self) -> DropSessionId {
        (self.platform_session as i64).into()
    }

    fn create_drop_event(&self, is_perform_drop: bool) -> NativeExtensionsResult<DropEvent> {
        let delegate = self.context_delegate()?;

        let session = self.platform_session;
        let location: CGPoint = unsafe { msg_send![session, locationInView: *self.context_view] };
        let allows_move: BOOL = unsafe { msg_send![session, allowsMoveOperation] };
        let allowed_operations = if allows_move == YES {
            vec![DropOperation::Copy, DropOperation::Move]
        } else {
            vec![DropOperation::Copy]
        };

        // local data
        let local_session: id = unsafe { msg_send![session, localDragSession] };
        let drag_contexts = delegate.get_platform_drag_contexts();
        let local_data = drag_contexts
            .iter()
            .map(|c| c.get_local_data(local_session))
            .find(|c| c.is_some())
            .flatten()
            .unwrap_or_default();

        // formats
        let mut items = Vec::new();
        let session_items: id = unsafe { msg_send![session, items] };
        for i in 0..unsafe { NSArray::count(session_items) } {
            let item: id = unsafe { NSArray::objectAtIndex(session_items, i) };
            let item_provider: id = unsafe { msg_send![item, itemProvider] };
            let mut formats = Vec::<String>::new();
            let identifiers: id = unsafe { msg_send![item_provider, registeredTypeIdentifiers] };
            for j in 0..unsafe { NSArray::count(identifiers) } {
                let identifier = unsafe { from_nsstring(NSArray::objectAtIndex(identifiers, j)) };
                if !formats.contains(&identifier) {
                    formats.push(identifier);
                }
            }
            items.push(DropItem {
                item_id: (item as i64).into(),
                formats,
                local_data: local_data.get(i as usize).cloned().unwrap_or(Value::Null),
            });
        }

        let accepted_operation = if is_perform_drop {
            Some(self.last_operation.get())
        } else {
            None
        };

        let reader = if is_perform_drop {
            let platform_reader = PlatformDataReader::new_with_drop_session_items(session_items)?;
            Some(delegate.register_platform_reader(self.context_id, platform_reader))
        } else {
            None
        };

        Ok(DropEvent {
            session_id: self.session_id(),
            location_in_view: location.into(),
            allowed_operations,
            items,
            accepted_operation,
            reader,
        })
    }

    fn session_did_update(self: &Rc<Self>) -> NativeExtensionsResult<id> {
        let delegate = self.context_delegate()?;
        let event = self.create_drop_event(false)?;

        let allows_move: BOOL = unsafe { msg_send![self.platform_session, allowsMoveOperation] };

        let session_clone = self.clone();
        delegate.send_drop_update(
            self.context_id,
            event,
            Box::new(move |res| {
                let mut res = res.ok_log().unwrap_or(DropOperation::None);
                if res == DropOperation::Move && allows_move == NO {
                    res = DropOperation::Copy;
                }
                session_clone.last_operation.replace(res);
            }),
        );

        let operation = self.last_operation.get().to_platform();

        let proposal: id = unsafe { msg_send![class!(UIDropProposal), alloc] };
        let proposal: id = unsafe { msg_send![proposal, initWithDropOperation: operation] };
        let () = unsafe { msg_send![proposal, autorelease] };

        Ok(proposal)
    }

    fn perform_drop(&self) -> NativeExtensionsResult<()> {
        let event = self.create_drop_event(true)?;
        let delegate = self.context_delegate()?;
        let done = Rc::new(Cell::new(false));
        let done_clone = done.clone();
        // TODO(knopp): Let user override default progress indicator
        let () =
            unsafe { msg_send![self.platform_session, setProgressIndicatorStyle: 0 as NSUInteger] };
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
            Context::get()
                .run_loop()
                .platform_run_loop
                .poll_once(&mut poll_session);
        }
        Ok(())
    }

    fn session_did_exit(&self) -> NativeExtensionsResult<()> {
        if let Some(delegate) = self.context_delegate.upgrade() {
            delegate.send_drop_leave(
                self.context_id,
                BaseDropEvent {
                    session_id: self.session_id(),
                },
            );
        }
        Ok(())
    }

    fn session_did_end(&self) -> NativeExtensionsResult<()> {
        if let Some(delegate) = self.context_delegate.upgrade() {
            delegate.send_drop_ended(
                self.context_id,
                BaseDropEvent {
                    session_id: self.session_id(),
                },
            );
        }
        Ok(())
    }

    const DEFAULT_FADE_OUT_DELAY: f64 = 0.8;
    const DEFAULT_FADE_OUT_DURATION: f64 = 0.3;

    fn get_drop_preview(
        &self,
        response: ItemPreview,
        original_size: Size,
        default: id, /* UITargetedDragPreview */
    ) -> id {
        let size = response
            .destination_image
            .as_ref()
            .map(|i| Size {
                width: (i.width as f64) / i.device_pixel_ratio.unwrap_or(1.0),
                height: (i.height as f64) / i.device_pixel_ratio.unwrap_or(1.0),
            })
            .unwrap_or(original_size);

        let center: CGPoint = response.destination_rect.center().into();
        let transform = unsafe {
            CGAffineTransformMakeScale(
                response.destination_rect.width / size.width,
                response.destination_rect.height / size.height,
            )
        };

        let view_container = unsafe {
            let bounds: CGRect = msg_send![*self.context_view, bounds];
            let container: id = msg_send![class!(UIView), alloc];
            let container = StrongPtr::new(msg_send![container, initWithFrame: bounds]);
            let () = msg_send![*container, setUserInteractionEnabled: NO];
            let () = msg_send![*self.context_view, addSubview: *container];

            let container_clone = container.clone();
            let animation_block = ConcreteBlock::new(move || {
                let () = msg_send![*container_clone, setAlpha: 0.0];
            });
            let animation_block = animation_block.copy();

            let () = msg_send![class!(UIView),
                         animateWithDuration: response.fade_out_duration.unwrap_or(Self::DEFAULT_FADE_OUT_DURATION)
                         delay: response.fade_out_delay.unwrap_or(Self::DEFAULT_FADE_OUT_DELAY)
                         options: 0 as NSUInteger
                         animations:&*animation_block completion:nil];

            container
        };
        self.view_containers
            .borrow_mut()
            .push(view_container.clone());

        let target = unsafe {
            let target: id = msg_send![class!(UIDragPreviewTarget), alloc];
            let () = msg_send![target, initWithContainer: *view_container center: center transform: transform];
            let () = msg_send![target, autorelease];
            target
        };

        match response.destination_image {
            Some(image) => unsafe {
                let image_view = image_view_from_data(image);

                let () = msg_send![*view_container, addSubview:*image_view];
                let frame: CGRect = response
                    .destination_rect
                    .translated(-100000.0, -100000.0)
                    .into();
                let () = msg_send![*image_view, setFrame: frame];

                let parameters: id = msg_send![class!(UIDragPreviewParameters), new];
                let () = msg_send![parameters, autorelease];

                let preview: id = msg_send![class!(UITargetedDragPreview), alloc];
                let () = msg_send![preview, initWithView:*image_view parameters:parameters target:target];
                let () = msg_send![preview, autorelease];

                preview
            },
            None => unsafe { msg_send![default, retargetedPreviewWithTarget: target] },
        }
    }

    fn preview_for_dropping_item(
        &self,
        item: id,    /* UIDragItem */
        default: id, /* UITargetedDragPreview */
    ) -> NativeExtensionsResult<id> {
        let delegate = self.context_delegate()?;

        let view: id = unsafe { msg_send![default, view] };
        let frame: CGRect = unsafe { msg_send![view, frame] };
        let original_size: Size = frame.size.into();
        let preview_promise = delegate.get_preview_for_item(
            self.context_id,
            ItemPreviewRequest {
                session_id: self.session_id(),
                item_id: (item as i64).into(),
                size: original_size.clone(),
                fade_out_delay: Self::DEFAULT_FADE_OUT_DELAY,
                fade_out_duration: Self::DEFAULT_FADE_OUT_DURATION,
            },
        );
        let mut poll_session = PollSession::new();
        loop {
            if let Some(result) = preview_promise.try_take() {
                match result {
                    PromiseResult::Ok { value } => match value.preview {
                        Some(preview) => {
                            return Ok(self.get_drop_preview(preview, original_size, default))
                        }
                        None => return Ok(nil),
                    },
                    PromiseResult::Cancelled => return Ok(nil),
                }
            }
            Context::get()
                .run_loop()
                .platform_run_loop
                .poll_once(&mut poll_session);
        }
    }
}

impl PlatformDropContext {
    pub fn new(
        id: i64,
        engine_handle: i64,
        delegate: Weak<dyn PlatformDropContextDelegate>,
    ) -> Self {
        let view = ENGINE_CONTEXT
            .with(|c| c.get_flutter_view(engine_handle))
            .expect("Failed to get FlutterView");
        Self {
            id,
            weak_self: Late::new(),
            view: unsafe { StrongPtr::retain(view) },
            delegate,
            interaction: Late::new(),
            interaction_delegate: Late::new(),
            sessions: RefCell::new(HashMap::new()),
        }
    }

    pub fn register_drop_formats(&self, _formats: &[String]) -> NativeExtensionsResult<()> {
        Ok(())
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self.clone());
        autoreleasepool(|| unsafe {
            let delegate: id = msg_send![*DELEGATE_CLASS, new];
            (*delegate).set_ivar("context", Weak::into_raw(weak_self) as *mut c_void);
            self.interaction_delegate.set(StrongPtr::new(delegate));
            let interaction: id = msg_send![class!(UIDropInteraction), alloc];
            let interaction: id = msg_send![interaction, initWithDelegate: delegate];
            self.interaction.set(StrongPtr::new(interaction));
            let () = msg_send![*self.view, addInteraction: interaction];
        });
    }

    fn get_session(&self, session: id) -> Rc<Session> {
        self.sessions
            .borrow_mut()
            .entry(session)
            .or_insert_with(|| {
                Rc::new(Session {
                    context_id: self.id,
                    context_delegate: self.delegate.clone(),
                    context_view: self.view.clone(),
                    platform_session: session,
                    last_operation: Cell::new(DropOperation::None),
                    view_containers: RefCell::new(Vec::new()),
                })
            })
            .clone()
    }

    fn session_did_update(&self, session: id) -> NativeExtensionsResult<id> {
        self.get_session(session).session_did_update()
    }

    fn perform_drop(&self, session: id) -> NativeExtensionsResult<()> {
        self.get_session(session).perform_drop()
    }

    fn session_did_exit(&self, session: id) -> NativeExtensionsResult<()> {
        self.get_session(session).session_did_exit()
    }

    fn session_did_end(&self, session: id) -> NativeExtensionsResult<()> {
        let session = self.sessions.borrow_mut().remove(&session);
        match session {
            Some(session) => session.session_did_end(),
            None => Ok(()),
        }
    }

    fn preview_for_dropping_item(&self, item: id, default: id) -> NativeExtensionsResult<id> {
        let session_for_item = self.sessions.borrow().iter().find_map(|(s, _)| {
            let items: id = unsafe { msg_send![*s, items] };
            let contains: BOOL = unsafe { msg_send![items, containsObject: item] };
            if contains == YES {
                Some(*s)
            } else {
                None
            }
        });
        match session_for_item {
            Some(session) => self
                .get_session(session)
                .preview_for_dropping_item(item, default),
            None => Ok(nil),
        }
    }
}

impl Drop for PlatformDropContext {
    fn drop(&mut self) {
        unsafe {
            let () = msg_send![*self.view, removeInteraction: **self.interaction];
        }
    }
}

fn with_state<F, FR, R>(this: id, callback: F, default: FR) -> R
where
    F: FnOnce(Rc<PlatformDropContext>) -> R,
    FR: FnOnce() -> R,
{
    unsafe {
        let context_ptr = {
            let context_ptr: *mut c_void = *(*this).get_ivar("context");
            context_ptr as *const PlatformDropContext
        };
        let this = ManuallyDrop::new(Weak::from_raw(context_ptr));
        let this = this.upgrade();
        match this {
            Some(this) => callback(this),
            None => default(),
        }
    }
}

extern "C" fn dealloc(this: &Object, _sel: Sel) {
    unsafe {
        let context_ptr = {
            let context_ptr: *mut c_void = *this.get_ivar("context");
            context_ptr as *const PlatformDropContext
        };
        Weak::from_raw(context_ptr);

        let superclass = superclass(this);
        let () = msg_send![super(this, superclass), dealloc];
    }
}

extern "C" fn can_handle_session(
    _this: &Object,
    _sel: Sel,
    _interaction: id,
    _session: id,
) -> BOOL {
    YES
}

extern "C" fn session_did_update(
    this: &mut Object,
    _sel: Sel,
    _interaction: id,
    session: id,
) -> id {
    with_state(
        this,
        |state| state.session_did_update(session).ok_log().unwrap_or(nil),
        || nil,
    )
}

extern "C" fn perform_drop(this: &mut Object, _sel: Sel, _interaction: id, session: id) {
    with_state(
        this,
        |state| state.perform_drop(session).ok_log().unwrap_or(()),
        || (),
    )
}

extern "C" fn session_did_exit(this: &mut Object, _sel: Sel, _interaction: id, session: id) {
    with_state(
        this,
        |state| state.session_did_exit(session).ok_log().unwrap_or(()),
        || (),
    )
}

extern "C" fn session_did_end(this: &mut Object, _sel: Sel, _interaction: id, session: id) {
    with_state(
        this,
        |state| state.session_did_end(session).ok_log().unwrap_or(()),
        || (),
    )
}

extern "C" fn preview_for_dropping_item(
    this: &mut Object,
    _sel: Sel,
    _interaction: id,
    item: id,
    default: id,
) -> id {
    with_state(
        this,
        |state| {
            state
                .preview_for_dropping_item(item, default)
                .ok_log()
                .unwrap_or(nil)
        },
        || nil,
    )
}

static DELEGATE_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("SNEDropInteractionDelegate", superclass).unwrap();
    decl.add_protocol(Protocol::get("UIDropInteractionDelegate").unwrap());
    decl.add_ivar::<*mut c_void>("context");
    decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
    decl.add_method(
        sel!(dropInteraction:canHandleSession:),
        can_handle_session as extern "C" fn(&Object, Sel, id, id) -> BOOL,
    );
    decl.add_method(
        sel!(dropInteraction:sessionDidUpdate:),
        session_did_update as extern "C" fn(&mut Object, Sel, id, id) -> id,
    );
    decl.add_method(
        sel!(dropInteraction:performDrop:),
        perform_drop as extern "C" fn(&mut Object, Sel, id, id),
    );
    decl.add_method(
        sel!(dropInteraction:sessionDidExit:),
        session_did_exit as extern "C" fn(&mut Object, Sel, id, id),
    );
    decl.add_method(
        sel!(dropInteraction:sessionDidEnd:),
        session_did_end as extern "C" fn(&mut Object, Sel, id, id),
    );
    decl.add_method(
        sel!(dropInteraction:previewForDroppingItem:withDefault:),
        preview_for_dropping_item as extern "C" fn(&mut Object, Sel, id, id, id) -> id,
    );
    decl.register()
});
