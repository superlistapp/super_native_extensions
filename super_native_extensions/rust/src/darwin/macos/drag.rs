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
    api_model::{DataProviderId, DragConfiguration, DragRequest, DropOperation},
    data_provider_manager::DataProviderHandle,
    drag_manager::{
        DataProviderEntry, DragSessionId, PlatformDragContextDelegate, PlatformDragContextId,
    },
    error::{NativeExtensionsError, NativeExtensionsResult},
    value_promise::PromiseResult,
};

use super::{
    drag_common::{DropOperationExt, NSDragOperation, NSDragOperationNone},
    util::{class_decl_from_name, flip_rect, ns_image_from_image_data},
};
use cocoa::{
    appkit::{
        NSApplication, NSEvent, NSEventPhase,
        NSEventType::{self, NSLeftMouseDown, NSMouseMoved, NSRightMouseDown},
        NSView, NSWindow,
    },
    base::{id, nil, BOOL, NO, YES},
    foundation::{NSArray, NSInteger, NSPoint, NSProcessInfo, NSRect},
};
use core_foundation::base::CFRelease;
use core_graphics::event::{CGEventField, CGEventType};

use irondash_engine_context::EngineContext;
use irondash_message_channel::Value;
use irondash_run_loop::{platform::PollSession, RunLoop};
use objc::{
    class, msg_send,
    rc::{autoreleasepool, StrongPtr},
    runtime::{Object, Sel},
    sel, sel_impl,
};

extern "C" {
    fn CGEventSetType(event: core_graphics::sys::CGEventRef, eventType: CGEventType);
    fn CGEventCreateCopy(event: core_graphics::sys::CGEventRef) -> core_graphics::sys::CGEventRef;
    fn CGEventSetIntegerValueField(
        event: core_graphics::sys::CGEventRef,
        field: CGEventField,
        value: i64,
    );
}

struct DragSession {
    session_id: DragSessionId,
    configuration: DragConfiguration,
    _data_provider_handles: Vec<Arc<DataProviderHandle>>,
}

pub struct PlatformDragContext {
    id: PlatformDragContextId,
    delegate: Weak<dyn PlatformDragContextDelegate>,
    pub view: StrongPtr,
    last_mouse_down_event: RefCell<Option<StrongPtr>>,
    last_mouse_up_event: RefCell<Option<StrongPtr>>,
    last_momentum_event: RefCell<Option<StrongPtr>>,
    sessions: RefCell<HashMap<NSInteger /* draggingSequenceNumber */, DragSession>>,
}

static ONCE: std::sync::Once = std::sync::Once::new();

thread_local! {
    pub static VIEW_TO_CONTEXT: RefCell<HashMap<id, Weak<PlatformDragContext>>> = RefCell::new(HashMap::new());
}

impl PlatformDragContext {
    pub fn new(
        id: PlatformDragContextId,
        engine_handle: i64,
        delegate: Weak<dyn PlatformDragContextDelegate>,
    ) -> NativeExtensionsResult<Self> {
        ONCE.call_once(prepare_flutter);
        let view = EngineContext::get()?.get_flutter_view(engine_handle)?;
        Ok(Self {
            id,
            delegate,
            view: unsafe { StrongPtr::retain(view) },
            last_mouse_down_event: RefCell::new(None),
            last_mouse_up_event: RefCell::new(None),
            last_momentum_event: RefCell::new(None),
            sessions: RefCell::new(HashMap::new()),
        })
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        VIEW_TO_CONTEXT.with(|v| {
            v.borrow_mut().insert(*self.view, weak_self);
        });
    }

    unsafe fn finish_momentum_events(&self) {
        let event = { self.last_momentum_event.borrow().as_ref().cloned() };
        // Unfinished momentum events will cause pan gesture recognizer
        // stuck since Flutter 3.3
        if let Some(event) = event {
            let phase = event.phase();
            if phase != NSEventPhase::NSEventPhaseNone
                && phase != NSEventPhase::NSEventPhaseEnded
                && phase != NSEventPhase::NSEventPhaseCancelled
            {
                let event = NSEvent::CGEvent(*event) as core_graphics::sys::CGEventRef;
                let event = CGEventCreateCopy(event);
                CGEventSetIntegerValueField(
                    event, //
                    99,    // kCGScrollWheelEventScrollPhase
                    NSEventPhase::NSEventPhaseEnded.bits() as i64,
                );

                let synthetized: id = msg_send![class!(NSEvent), eventWithCGEvent: event];
                CFRelease(event as *mut _);

                let window: id = msg_send![*self.view, window];
                let _: () = msg_send![window, sendEvent: synthetized];
            }
        }
    }

    pub unsafe fn synthetize_mouse_up_event(&self) {
        self.finish_momentum_events();

        if let Some(event) = self.last_mouse_down_event.borrow().as_ref().cloned() {
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

    pub fn needs_combined_drag_image() -> bool {
        false
    }

    pub async fn start_drag(
        &self,
        request: DragRequest,
        mut providers: HashMap<DataProviderId, DataProviderEntry>,
        session_id: DragSessionId,
    ) -> NativeExtensionsResult<()> {
        autoreleasepool(|| unsafe {
            self.synthetize_mouse_up_event();

            let mut dragging_items = Vec::<id>::new();
            let mut data_provider_handles = Vec::<_>::new();

            for item in &request.configuration.items {
                let provider = providers
                    .remove(&item.data_provider_id)
                    .expect("Provider missing");
                let writer_item =
                    provider
                        .provider
                        .create_writer(provider.handle.clone(), false, true);
                data_provider_handles.push(provider.handle);

                let dragging_item: id = msg_send![class!(NSDraggingItem), alloc];
                let dragging_item: id =
                    msg_send![dragging_item, initWithPasteboardWriter: *writer_item];
                let dragging_item: id = msg_send![dragging_item, autorelease];

                let image = &item.image;
                let mut rect: NSRect = image.rect.clone().into();
                flip_rect(*self.view, &mut rect);
                let snapshot = ns_image_from_image_data(vec![image.image_data.clone()]);

                let () = msg_send![dragging_item, setDraggingFrame:rect contents:*snapshot];
                dragging_items.push(dragging_item);
            }
            let event = self
                .last_mouse_down_event
                .borrow()
                .as_ref()
                .cloned()
                .ok_or(NativeExtensionsError::MouseEventNotFound)?;
            let dragging_items = NSArray::arrayWithObjects(nil, &dragging_items);

            let app = NSApplication::sharedApplication(nil);
            let () = msg_send![app, preventWindowOrdering];

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
            let dragging_sequence_number: NSInteger = msg_send![session, draggingSequenceNumber];
            self.sessions.borrow_mut().insert(
                dragging_sequence_number,
                DragSession {
                    session_id,
                    configuration: request.configuration,
                    _data_provider_handles: data_provider_handles,
                },
            );
            Ok(())
        })
    }

    fn on_mouse_down(&self, event: id) {
        unsafe {
            self.last_mouse_down_event
                .replace(Some(StrongPtr::retain(event)));
        }
    }

    fn on_mouse_up(&self, event: id) {
        unsafe {
            self.last_mouse_up_event
                .replace(Some(StrongPtr::retain(event)));
        }
    }

    fn on_right_mouse_down(&self, event: id) {
        unsafe {
            self.last_mouse_down_event
                .replace(Some(StrongPtr::retain(event)));
        }
    }

    fn on_right_mouse_up(&self, event: id) {
        unsafe {
            self.last_mouse_up_event
                .replace(Some(StrongPtr::retain(event)));
        }
    }

    fn on_momentum_event(&self, event: id) {
        unsafe {
            self.last_momentum_event
                .replace(Some(StrongPtr::retain(event)));
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
            NSEvent::eventType(event) == NSEventType::NSKeyDown
                && NSEvent::keyCode(event) == K_VKESCAPE
        };

        let dragging_sequence_number: NSInteger =
            unsafe { msg_send![session, draggingSequenceNumber] };
        let session = self
            .sessions
            .borrow_mut()
            .remove(&dragging_sequence_number)
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
        RunLoop::current()
            .schedule(Duration::from_secs(3), move || {
                let _data_provider_handles = session._data_provider_handles;
            })
            .detach();
    }

    pub fn drag_moved(&self, session: id, point: NSPoint) {
        let sessions = self.sessions.borrow();
        let dragging_sequence_number: NSInteger =
            unsafe { msg_send![session, draggingSequenceNumber] };
        let session = sessions
            .get(&dragging_sequence_number)
            .expect("Drag session unexpectedly missing");
        if let Some(delegate) = self.delegate.upgrade() {
            delegate.drag_session_did_move_to_location(self.id, session.session_id, point.into());
        }
    }

    pub fn should_delay_window_ordering(&self, event: id) -> bool {
        if unsafe { NSEvent::eventType(event) == NSEventType::NSLeftMouseDown } {
            let location: NSPoint = unsafe { msg_send![event, locationInWindow] };
            let location: NSPoint =
                unsafe { NSView::convertPoint_fromView_(*self.view, location, nil) };
            if let Some(delegate) = self.delegate.upgrade() {
                let is_draggable_promise = delegate.is_location_draggable(self.id, location.into());
                let mut poll_session = PollSession::new();
                loop {
                    if let Some(result) = is_draggable_promise.try_take() {
                        match result {
                            PromiseResult::Ok { value } => return value,
                            PromiseResult::Cancelled => return false,
                        }
                    }
                    RunLoop::current()
                        .platform_run_loop
                        .poll_once(&mut poll_session);
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    fn source_operation_mask_for_dragging_context(
        &self,
        session: id,
        _context: NSInteger,
    ) -> NSDragOperation {
        let sessions = self.sessions.borrow();
        let dragging_sequence_number: NSInteger =
            unsafe { msg_send![session, draggingSequenceNumber] };
        let session = sessions.get(&dragging_sequence_number);
        match session {
            Some(sessions) => {
                let mut res = NSDragOperationNone;
                for operation in &sessions.configuration.allowed_operations {
                    res |= operation.to_platform();
                }
                res
            }
            None => NSDragOperationNone,
        }
    }

    pub fn get_local_data(&self, dragging_sequence_number: NSInteger) -> Option<Vec<Value>> {
        let sessions = self.sessions.borrow();
        sessions
            .get(&dragging_sequence_number)
            .map(|s| s.configuration.get_local_data())
    }

    pub fn get_local_data_for_session_id(
        &self,
        session_id: DragSessionId,
    ) -> NativeExtensionsResult<Vec<Value>> {
        let sessions = self.sessions.borrow();
        let session = sessions
            .iter()
            .find_map(|s| {
                if s.1.session_id == session_id {
                    Some(s.1)
                } else {
                    None
                }
            })
            .ok_or(NativeExtensionsError::DragSessionNotFound)?;
        Ok(session.configuration.get_local_data())
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

        // Custom mouseDown implementation will cause AppKit to query `mouseDownCanMoveWindow`
        // to determine draggable window region. If this does not return YES then
        // dragging with transparent titlebar + full size content view won't work:
        // https://github.com/superlistapp/super_native_extensions/issues/42
        class.add_method(
            sel!(mouseDownCanMoveWindow),
            mouse_down_can_move_window as extern "C" fn(&mut Object, Sel) -> BOOL,
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
        class.add_method(
            sel!(rightMouseDown:),
            right_mouse_down as extern "C" fn(&mut Object, Sel, id) -> (),
        );
        class.add_method(
            sel!(rightMouseUp:),
            right_mouse_up as extern "C" fn(&mut Object, Sel, id) -> (),
        );
        class.add_method(
            sel!(scrollWheel:),
            scroll_wheel as extern "C" fn(&mut Object, Sel, id) -> (),
        );
        class.add_method(
            sel!(magnifyWithEvent:),
            magnify_with_event as extern "C" fn(&mut Object, Sel, id) -> (),
        );
        class.add_method(
            sel!(rotateWithEvent:),
            rotate_with_event as extern "C" fn(&mut Object, Sel, id) -> (),
        );
        class.add_method(
            sel!(shouldDelayWindowOrderingForEvent:),
            should_delay_window_ordering as extern "C" fn(&mut Object, Sel, id) -> BOOL,
        )
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

extern "C" fn mouse_down_can_move_window(_this: &mut Object, _sel: Sel) -> BOOL {
    YES
}

extern "C" fn mouse_down(this: &mut Object, _sel: Sel, event: id) {
    with_state(this, |state| state.on_mouse_down(event), || ());

    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), mouseDown: event];
    }
}

extern "C" fn mouse_up(this: &mut Object, _sel: Sel, event: id) {
    with_state(this, |state| state.on_mouse_up(event), || ());

    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), mouseUp: event];
    }
}

extern "C" fn right_mouse_down(this: &mut Object, _sel: Sel, event: id) {
    with_state(this, |state| state.on_right_mouse_down(event), || ());

    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), rightMouseDown: event];
    }
}

extern "C" fn right_mouse_up(this: &mut Object, _sel: Sel, event: id) {
    with_state(this, |state| state.on_right_mouse_up(event), || ());

    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), rightMouseUp: event];
    }
}

extern "C" fn scroll_wheel(this: &mut Object, _sel: Sel, event: id) {
    with_state(this, |state| state.on_momentum_event(event), || ());
    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), scrollWheel: event];
    }
}

extern "C" fn magnify_with_event(this: &mut Object, _sel: Sel, event: id) {
    with_state(this, |state| state.on_momentum_event(event), || ());

    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), magnifyWithEvent: event];
    }
}

extern "C" fn rotate_with_event(this: &mut Object, _sel: Sel, event: id) {
    with_state(this, |state| state.on_momentum_event(event), || ());

    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), rotateWithEvent: event];
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

extern "C" fn should_delay_window_ordering(this: &mut Object, _: Sel, event: id) -> BOOL {
    with_state(
        this,
        move |state| {
            if state.should_delay_window_ordering(event) {
                YES
            } else {
                NO
            }
        },
        || YES,
    )
}
