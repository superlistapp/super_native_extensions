use core::panic;
use std::{
    cell::RefCell,
    collections::HashMap,
    os::raw::c_ushort,
    rc::{Rc, Weak},
    sync::Arc,
    time::Duration,
};

use crate::{
    api_model::{DataProviderId, DragRequest, DropOperation},
    data_provider_manager::DataProviderHandle,
    drag_manager::{DataProviderEntry, DragSessionId, PlatformDragContextDelegate},
    error::NativeExtensionsResult,
};

use super::{
    drag_common::{DropOperationExt, NSDragOperation, NSDragOperationNone},
    util::{class_decl_from_name, flip_rect, ns_image_from_image_data},
};
use cocoa::{
    appkit::{
        NSApplication, NSEvent,
        NSEventType::{self, NSLeftMouseDown, NSMouseMoved, NSRightMouseDown},
        NSWindow,
    },
    base::{id, nil, NO, YES},
    foundation::{NSArray, NSInteger, NSPoint, NSProcessInfo, NSRect},
};
use core_foundation::base::CFRelease;
use core_graphics::event::CGEventType;

use nativeshell_core::Context;
use objc::{
    class, msg_send,
    rc::{autoreleasepool, StrongPtr},
    runtime::{Object, Sel},
    sel, sel_impl,
};

extern "C" {
    fn CGEventSetType(event: core_graphics::sys::CGEventRef, eventType: CGEventType);
    fn CGEventCreateCopy(event: core_graphics::sys::CGEventRef) -> core_graphics::sys::CGEventRef;
}

struct DragSession {
    session_id: DragSessionId,
    allowed_operations: Vec<DropOperation>,
    _data_provider_handles: Vec<Arc<DataProviderHandle>>,
}

pub struct PlatformDragContext {
    id: i64,
    delegate: Weak<dyn PlatformDragContextDelegate>,
    view: StrongPtr,
    last_mouse_down: RefCell<Option<StrongPtr>>,
    last_mouse_up: RefCell<Option<StrongPtr>>,
    sessions: RefCell<HashMap<id, DragSession>>,
}

static ONCE: std::sync::Once = std::sync::Once::new();

thread_local! {
    pub static VIEW_TO_CONTEXT: RefCell<HashMap<id, Weak<PlatformDragContext>>> = RefCell::new(HashMap::new());
}

impl PlatformDragContext {
    pub fn new(id: i64, view_handle: i64, delegate: Weak<dyn PlatformDragContextDelegate>) -> Self {
        ONCE.call_once(prepare_flutter);
        Self {
            id,
            delegate,
            view: unsafe { StrongPtr::retain(view_handle as *mut _) },
            last_mouse_down: RefCell::new(None),
            last_mouse_up: RefCell::new(None),
            sessions: RefCell::new(HashMap::new()),
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        VIEW_TO_CONTEXT.with(|v| {
            v.borrow_mut().insert(*self.view, weak_self);
        });
    }

    unsafe fn synthetize_mouse_up_event(&self) {
        if let Some(event) = self.last_mouse_down.borrow().as_ref().clone().cloned() {
            let opposite = match event.eventType() {
                NSLeftMouseDown => CGEventType::LeftMouseUp,
                NSRightMouseDown => CGEventType::RightMouseUp,
                _ => return,
            };

            let event = NSEvent::CGEvent(*event) as core_graphics::sys::CGEventRef;
            let event = CGEventCreateCopy(event);
            CGEventSetType(event, opposite);

            let synthetized: id = msg_send![class!(NSEvent), eventWithCGEvent: event];
            CFRelease(event as *mut _);

            let window: id = msg_send![*self.view, window];
            let _: () = msg_send![window, sendEvent: synthetized];
        }
    }

    pub async fn start_drag(
        &self,
        request: DragRequest,
        mut providers: HashMap<DataProviderId, DataProviderEntry>,
        session_id: DragSessionId,
    ) -> NativeExtensionsResult<()> {
        autoreleasepool(|| unsafe {
            self.synthetize_mouse_up_event();

            let image = &request.configuration.items.first().unwrap().image;

            let mut rect: NSRect = image.source_rect.clone().into();
            flip_rect(*self.view, &mut rect);
            let mut dragging_items = Vec::<id>::new();
            let mut first = true;
            let snapshot = ns_image_from_image_data(vec![image.image_data.clone()]);
            let mut data_provider_handles = Vec::<_>::new();

            for item in request.configuration.items {
                let provider = providers
                    .remove(&item.data_provider_id)
                    .expect("Provider missing");
                let writer_item = provider
                    .provider
                    .create_writer(provider.handle.clone(), false);
                data_provider_handles.push(provider.handle);

                let dragging_item: id = msg_send![class!(NSDraggingItem), alloc];
                let dragging_item: id =
                    msg_send![dragging_item, initWithPasteboardWriter: *writer_item];
                let dragging_item: id = msg_send![dragging_item, autorelease];
                let () = msg_send![dragging_item,
                   setDraggingFrame:rect
                   contents:if first {*snapshot } else {nil}
                ];
                dragging_items.push(dragging_item);
                first = false;
            }
            let event = self.last_mouse_down.borrow().as_ref().cloned().unwrap();
            let dragging_items = NSArray::arrayWithObjects(nil, &dragging_items);

            let session: id = msg_send![*self.view,
                beginDraggingSessionWithItems:dragging_items
                event:*event
                source:*self.view
            ];
            let animates = if request
                .configuration
                .animates_to_starting_position_on_cancel_or_fail
            {
                YES
            } else {
                NO
            };
            let () = msg_send![
                session,
                setAnimatesToStartingPositionsOnCancelOrFail: animates
            ];
            self.sessions.borrow_mut().insert(
                session,
                DragSession {
                    session_id,
                    allowed_operations: request.configuration.allowed_operations,
                    _data_provider_handles: data_provider_handles,
                },
            );
        });
        Ok(())
    }

    fn on_mouse_down(&self, event: id) {
        unsafe {
            self.last_mouse_down.replace(Some(StrongPtr::retain(event)));
        }
    }

    fn on_mouse_up(&self, event: id) {
        unsafe {
            self.last_mouse_up.replace(Some(StrongPtr::retain(event)));
        }
    }

    fn synthetize_mouse_move_if_needed(&self) {
        unsafe {
            fn system_uptime() -> f64 {
                unsafe {
                    let info = NSProcessInfo::processInfo(nil);
                    msg_send![info, systemUptime]
                }
            }
            let location = NSEvent::mouseLocation(nil);
            let window: id = msg_send![*self.view, window];
            let window_frame = NSWindow::frame(window);
            let content_rect = NSWindow::contentRectForFrameRect_(window, window_frame);
            let tail = NSPoint {
                x: content_rect.origin.x + content_rect.size.width,
                y: content_rect.origin.y + content_rect.size.height,
            };
            if location.x > content_rect.origin.x
                && location.x < tail.x
                && location.y > content_rect.origin.y
                && location.y < tail.y
            {
                let location: NSPoint = msg_send![window, convertPointFromScreen: location];
                let event: id = msg_send![class!(NSEvent), mouseEventWithType: NSMouseMoved
                    location:location
                    modifierFlags:NSEvent::modifierFlags(nil)
                    timestamp: system_uptime()
                    windowNumber:0
                    context:nil
                    eventNumber:0
                    clickCount:1
                    pressure:0
                ];
                let () = msg_send![window, sendEvent: event];
            }
        }
    }

    pub fn drag_ended(&self, session: id, _point: NSPoint, operation: NSDragOperation) {
        let user_cancelled = unsafe {
            let app = NSApplication::sharedApplication(nil);
            let event: id = msg_send![app, currentEvent];
            const K_VKESCAPE: c_ushort = 0x35;
            if NSEvent::eventType(event) == NSEventType::NSKeyDown
                && NSEvent::keyCode(event) == K_VKESCAPE
            {
                true
            } else {
                false
            }
        };

        let session = self
            .sessions
            .borrow_mut()
            .remove(&session)
            .expect("Drag session unexpectedly missing");

        let operations = DropOperation::from_platform_mask(operation);
        // there might be multiple operation, use the order from from_platform_mask
        let operation = operations.into_iter().next().unwrap_or(DropOperation::None);
        let operation = if operation == DropOperation::None && user_cancelled {
            DropOperation::UserCancelled
        } else {
            operation
        };
        if let Some(delegate) = self.delegate.upgrade() {
            delegate.drag_session_did_end_with_operation(self.id, session.session_id, operation);
        }

        // Fix hover after mouse move
        self.synthetize_mouse_move_if_needed();
        // Wait a bit to ensure drop site had enough time to request data.
        // Note that for file promises the drop notifier lifetime is extended
        // until the promise is fulfilled in data source.
        Context::get()
            .run_loop()
            .schedule(Duration::from_secs(3), move || {
                let _data_provider_handles = session._data_provider_handles;
            })
            .detach();
    }

    pub fn drag_moved(&self, session: id, point: NSPoint) {
        let sessions = self.sessions.borrow();
        let session = sessions
            .get(&session)
            .expect("Drag session unexpectedly missing");
        if let Some(delegate) = self.delegate.upgrade() {
            delegate.drag_session_did_move_to_location(self.id, session.session_id, point.into());
        }
    }

    fn source_operation_mask_for_dragging_context(
        &self,
        session: id,
        _context: NSInteger,
    ) -> NSDragOperation {
        let sessions = self.sessions.borrow();
        let session = sessions.get(&session);
        match session {
            Some(sessions) => {
                let mut res = NSDragOperationNone;
                for operation in &sessions.allowed_operations {
                    res |= operation.to_platform();
                }
                res
            }
            None => NSDragOperationNone,
        }
    }
}

impl Drop for PlatformDragContext {
    fn drop(&mut self) {
        VIEW_TO_CONTEXT.with(|v| {
            v.borrow_mut().remove(&*self.view);
        });
    }
}

//
//
//

fn prepare_flutter() {
    unsafe {
        let mut class = class_decl_from_name("FlutterView");

        class.add_method(
            sel!(draggingSession:sourceOperationMaskForDraggingContext:),
            source_operation_mask_for_dragging_context
                as extern "C" fn(&mut Object, Sel, id, NSInteger) -> NSDragOperation,
        );

        class.add_method(
            sel!(draggingSession:endedAtPoint:operation:),
            dragging_session_ended_at_point
                as extern "C" fn(&mut Object, Sel, id, NSPoint, NSDragOperation),
        );

        class.add_method(
            sel!(draggingSession:movedToPoint:),
            dragging_session_moved_to_point as extern "C" fn(&mut Object, Sel, id, NSPoint),
        );

        // Flutter implements mouseDown: on FlutterViewController, so we can add
        // implementation to FlutterView, intercept the event and call super.
        // If this changes and Flutter implements mouseDown: directly on
        // FlutterView, we could either swizzle the method or implement it on
        // FlutterViewWrapper.
        class.add_method(
            sel!(mouseDown:),
            mouse_down as extern "C" fn(&mut Object, Sel, id) -> (),
        );
        class.add_method(
            sel!(mouseUp:),
            mouse_up as extern "C" fn(&mut Object, Sel, id) -> (),
        );
    }
}

fn with_state<F, FR, R>(this: id, callback: F, default: FR) -> R
where
    F: FnOnce(Rc<PlatformDragContext>) -> R,
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

extern "C" fn mouse_down(this: &mut Object, _sel: Sel, event: id) -> () {
    with_state(this, |state| state.on_mouse_down(event), || ());

    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), mouseDown: event];
    }
}

extern "C" fn mouse_up(this: &mut Object, _sel: Sel, event: id) -> () {
    with_state(this, |state| state.on_mouse_up(event), || ());

    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), mouseUp: event];
    }
}

extern "C" fn source_operation_mask_for_dragging_context(
    this: &mut Object,
    _: Sel,
    session: id,
    context: NSInteger,
) -> NSDragOperation {
    with_state(
        this,
        move |state| state.source_operation_mask_for_dragging_context(session, context),
        || NSDragOperationNone,
    )
}

extern "C" fn dragging_session_ended_at_point(
    this: &mut Object,
    _: Sel,
    session: id,
    point: NSPoint,
    operation: NSDragOperation,
) {
    with_state(
        this,
        move |state| state.drag_ended(session, point, operation),
        || (),
    )
}

extern "C" fn dragging_session_moved_to_point(
    this: &mut Object,
    _: Sel,
    session: id,
    point: NSPoint,
) {
    with_state(this, move |state| state.drag_moved(session, point), || ())
}
