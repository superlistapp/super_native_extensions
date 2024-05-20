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
    drag_common::DropOperationExt,
    util::{class_builder_from_name, flip_rect, ns_image_from_image_data, EventExt},
};

use core_foundation::base::CFRelease;
use core_graphics::event::{CGEventField, CGEventType};

use irondash_engine_context::EngineContext;
use irondash_message_channel::Value;
use irondash_run_loop::{platform::PollSession, RunLoop};
use objc2_app_kit::{
    NSApplication, NSDragOperation, NSDraggingContext, NSDraggingItem, NSDraggingSession, NSEvent,
    NSEventPhase, NSEventType, NSView,
};
use objc2_foundation::{MainThreadMarker, NSArray, NSPoint, NSProcessInfo, NSRect};

use objc2::{
    class,
    ffi::NSInteger,
    msg_send,
    rc::Id,
    runtime::{Bool, Sel},
    sel, ClassType,
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
    pub view: Id<NSView>,
    last_mouse_down_event: RefCell<Option<Id<NSEvent>>>,
    last_mouse_up_event: RefCell<Option<Id<NSEvent>>>,
    last_scroll_event: RefCell<Option<Id<NSEvent>>>,
    sessions: RefCell<HashMap<isize /* draggingSequenceNumber */, DragSession>>,
    main_thread_marker: MainThreadMarker,
}

static ONCE: std::sync::Once = std::sync::Once::new();

thread_local! {
    pub static VIEW_TO_CONTEXT: RefCell<HashMap<Id<NSView>, Weak<PlatformDragContext>>> = RefCell::new(HashMap::new());
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
            view: unsafe { Id::cast(view) },
            last_mouse_down_event: RefCell::new(None),
            last_mouse_up_event: RefCell::new(None),
            last_scroll_event: RefCell::new(None),
            sessions: RefCell::new(HashMap::new()),
            main_thread_marker: MainThreadMarker::new().unwrap(),
        })
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        VIEW_TO_CONTEXT.with(|v| {
            v.borrow_mut().insert(self.view.clone(), weak_self);
        });
    }

    unsafe fn finish_scroll_events(&self) {
        let event = { self.last_scroll_event.borrow().as_ref().cloned() };
        // Unfinished momentum events will cause pan gesture recognizer
        // stuck since Flutter 3.3
        if let Some(event) = event {
            let phase = event.phase();
            if phase != NSEventPhase::None
                && phase != NSEventPhase::Ended
                && phase != NSEventPhase::Cancelled
            {
                let event = event.CGEvent();
                let event = CGEventCreateCopy(event);
                CGEventSetIntegerValueField(
                    event, //
                    99,    // kCGScrollWheelEventScrollPhase
                    NSEventPhase::Ended.0 as i64,
                );

                let synthesized = NSEvent::withCGEvent(event);
                CFRelease(event as *mut _);

                let window = self.view.window();
                if let Some(window) = window {
                    window.sendEvent(&synthesized);
                }
            }
        }
    }

    pub unsafe fn synthesize_mouse_up_event(&self) {
        self.finish_scroll_events();

        if let Some(event) = self.last_mouse_down_event.borrow().as_ref().cloned() {
            #[allow(non_upper_case_globals)]
            let opposite = match event.r#type() {
                NSEventType::LeftMouseDown => CGEventType::LeftMouseUp,
                NSEventType::RightMouseDown => CGEventType::RightMouseUp,
                _ => return,
            };

            let event = event.CGEvent();
            let event = CGEventCreateCopy(event);
            CGEventSetType(event, opposite);

            let synthesized = NSEvent::withCGEvent(event);
            CFRelease(event as *mut _);

            let window = self.view.window();
            if let Some(window) = window {
                window.sendEvent(&synthesized);
            }
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
        unsafe { self.synthesize_mouse_up_event() };

        let mut dragging_items = Vec::<Id<NSDraggingItem>>::new();
        let mut data_provider_handles = Vec::<_>::new();

        for item in &request.configuration.items {
            let provider = providers
                .remove(&item.data_provider_id)
                .expect("Provider missing");
            let writer_item = provider
                .provider
                .create_writer(provider.handle.clone(), false, true);
            data_provider_handles.push(provider.handle);

            let dragging_item = NSDraggingItem::alloc();
            let dragging_item = unsafe {
                NSDraggingItem::initWithPasteboardWriter(dragging_item, &Id::cast(writer_item))
            };

            let image = &item.image;
            let mut rect: NSRect = image.rect.clone().into();
            flip_rect(&self.view, &mut rect);
            let snapshot = ns_image_from_image_data(vec![image.image_data.clone()]);

            unsafe { dragging_item.setDraggingFrame_contents(rect, Some(&snapshot)) };
            dragging_items.push(dragging_item);
        }
        let event = self
            .last_mouse_down_event
            .borrow()
            .as_ref()
            .cloned()
            .ok_or(NativeExtensionsError::MouseEventNotFound)?;

        unsafe {
            NSApplication::sharedApplication(self.main_thread_marker).preventWindowOrdering()
        };

        let dragging_items = NSArray::from_vec(dragging_items);
        let session = unsafe {
            self.view.beginDraggingSessionWithItems_event_source(
                &dragging_items,
                &event,
                &Id::cast(self.view.clone()),
            )
        };

        let animates = request
            .configuration
            .animates_to_starting_position_on_cancel_or_fail;

        unsafe { session.setAnimatesToStartingPositionsOnCancelOrFail(animates) };

        let dragging_sequence_number = unsafe { session.draggingSequenceNumber() };
        self.sessions.borrow_mut().insert(
            dragging_sequence_number,
            DragSession {
                session_id,
                configuration: request.configuration,
                _data_provider_handles: data_provider_handles,
            },
        );
        Ok(())
    }

    fn on_mouse_down(&self, event: Id<NSEvent>) {
        self.last_mouse_down_event.replace(Some(event));
    }

    fn on_mouse_up(&self, event: Id<NSEvent>) {
        self.last_mouse_up_event.replace(Some(event));
    }

    fn on_right_mouse_down(&self, event: Id<NSEvent>) {
        self.last_mouse_down_event.replace(Some(event));
    }

    fn on_right_mouse_up(&self, event: Id<NSEvent>) {
        self.last_mouse_up_event.replace(Some(event));
    }

    /// Returns false if the event should be discarded.
    fn on_scroll_event(&self, event: Id<NSEvent>) -> bool {
        let last_event = self.last_scroll_event.borrow().clone();

        // println!(">> event {:?}", event);
        // println!(">> last_event {:?}", last_event);

        if let Some(last_event) = last_event.as_ref() {
            // For unknown reasons after closing context menu cocoa sometimes sends events in past.
            if unsafe { event.timestamp() } < unsafe { last_event.timestamp() } {
                return false;
            }
        }

        let event_phase = unsafe { event.phase() };
        let last_event_phase = last_event
            .map(|e| unsafe { e.phase() })
            .unwrap_or(NSEventPhase::None);

        // Flutter engine is very particular about the order of events. Anything deviating from
        // the expected order will trigger an assertion. The issue here is that with context menu
        // or drag & drop sometimes events are missing which leads to a crash on assertion on next
        // event. In order to prevent that the event flow needs to be sanitized.
        if (event_phase == NSEventPhase::Ended || event_phase == NSEventPhase::Cancelled)
            && (last_event_phase == NSEventPhase::Ended
                || last_event_phase == NSEventPhase::Cancelled
                || last_event_phase == NSEventPhase::None)
        {
            // End or cancelled event should only follow phase may begin, began or changed.
            return false;
        }
        if event_phase == NSEventPhase::Changed
            && (last_event_phase == NSEventPhase::Ended
                || last_event_phase == NSEventPhase::None
                || last_event_phase == NSEventPhase::Cancelled)
        {
            // Missing event began. Instead of throwing it away synthesize event with phase began
            // so that tne engine can process it.
            unsafe {
                let event = event.CGEvent();
                let event = CGEventCreateCopy(event);
                CGEventSetIntegerValueField(
                    event, //
                    99,    // kCGScrollWheelEventScrollPhase
                    NSEventPhase::Began.0 as i64,
                );
                let synthesized = NSEvent::withCGEvent(event);
                CFRelease(event as *mut _);

                let window = self.view.window();
                if let Some(window) = window {
                    window.sendEvent(&synthesized);
                }
            }
        }

        self.last_scroll_event.replace(Some(event));
        true
    }

    fn synthesize_mouse_move_if_needed(&self) {
        unsafe {
            fn system_uptime() -> f64 {
                unsafe {
                    let info = NSProcessInfo::processInfo();
                    info.systemUptime()
                }
            }
            let location = NSEvent::mouseLocation();
            let window = self.view.window();
            let Some(window) = window else { return };
            let window_frame = window.frame();
            let content_rect = window.contentRectForFrameRect(window_frame);
            let tail = NSPoint {
                x: content_rect.origin.x + content_rect.size.width,
                y: content_rect.origin.y + content_rect.size.height,
            };
            if location.x > content_rect.origin.x
                && location.x < tail.x
                && location.y > content_rect.origin.y
                && location.y < tail.y
            {
                let location: NSPoint = window.convertPointFromScreen(location);
                let event = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
                    NSEventType::MouseMoved, location, NSEvent::modifierFlags_class(), system_uptime(), 0, None, 0, 1, 0.0);
                let event = event.unwrap();
                window.sendEvent(&event);
            }
        }
    }

    pub fn drag_ended(
        &self,
        session: &NSDraggingSession,
        _point: NSPoint,
        operation: NSDragOperation,
    ) {
        let user_cancelled = unsafe {
            let app = NSApplication::sharedApplication(self.main_thread_marker);
            let event = app.currentEvent();
            match event {
                Some(event) => {
                    const K_VKESCAPE: c_ushort = 0x35;
                    event.r#type() == NSEventType::KeyDown && event.keyCode() == K_VKESCAPE
                }
                None => false,
            }
        };

        let dragging_sequence_number = unsafe { session.draggingSequenceNumber() };
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
        self.synthesize_mouse_move_if_needed();
        // Wait a bit to ensure drop site had enough time to request data.
        // Note that for file promises the drop notifier lifetime is extended
        // until the promise is fulfilled in data source.
        RunLoop::current()
            .schedule(Duration::from_secs(3), move || {
                let _data_provider_handles = session._data_provider_handles;
            })
            .detach();
    }

    pub fn drag_moved(&self, session: &NSDraggingSession, point: NSPoint) {
        let sessions = self.sessions.borrow();
        let dragging_sequence_number = unsafe { session.draggingSequenceNumber() };
        let session = sessions
            .get(&dragging_sequence_number)
            .expect("Drag session unexpectedly missing");
        if let Some(delegate) = self.delegate.upgrade() {
            delegate.drag_session_did_move_to_location(self.id, session.session_id, point.into());
        }
    }

    pub fn should_delay_window_ordering(&self, event: &NSEvent) -> bool {
        if unsafe { event.r#type() } == NSEventType::LeftMouseDown {
            let location: NSPoint = unsafe { event.locationInWindow() };
            let location: NSPoint = self.view.convertPoint_fromView(location, None);
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
        session: &NSDraggingSession,
        _context: NSDraggingContext,
    ) -> NSDragOperation {
        let sessions = self.sessions.borrow();
        let dragging_sequence_number = unsafe { session.draggingSequenceNumber() };
        let session = sessions.get(&dragging_sequence_number);
        match session {
            Some(sessions) => {
                let mut res = NSDragOperation::None.0;
                for operation in &sessions.configuration.allowed_operations {
                    res |= operation.to_platform().0;
                }
                NSDragOperation(res)
            }
            None => NSDragOperation::None,
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
        let mut class = class_builder_from_name("FlutterView");

        class.add_method(
            sel!(draggingSession:sourceOperationMaskForDraggingContext:),
            source_operation_mask_for_dragging_context
                as extern "C" fn(_, _, _, _) -> NSDragOperation,
        );

        class.add_method(
            sel!(draggingSession:endedAtPoint:operation:),
            dragging_session_ended_at_point as extern "C" fn(_, _, _, _, _),
        );

        class.add_method(
            sel!(draggingSession:movedToPoint:),
            dragging_session_moved_to_point as extern "C" fn(_, _, _, _),
        );

        // Custom mouseDown implementation will cause AppKit to query `mouseDownCanMoveWindow`
        // to determine draggable window region. If this does not return YES then
        // dragging with transparent titlebar + full size content view won't work:
        // https://github.com/superlistapp/super_native_extensions/issues/42
        class.add_method(
            sel!(mouseDownCanMoveWindow),
            mouse_down_can_move_window as extern "C" fn(_, _) -> _,
        );

        // Flutter implements mouseDown: on FlutterViewController, so we can add
        // implementation to FlutterView, intercept the event and call super.
        // If this changes and Flutter implements mouseDown: directly on
        // FlutterView, we could either swizzle the method or implement it on
        // FlutterViewWrapper.
        class.add_method(sel!(mouseDown:), mouse_down as extern "C" fn(_, _, _));
        class.add_method(sel!(mouseUp:), mouse_up as extern "C" fn(_, _, _));
        class.add_method(
            sel!(rightMouseDown:),
            right_mouse_down as extern "C" fn(_, _, _),
        );
        class.add_method(
            sel!(rightMouseUp:),
            right_mouse_up as extern "C" fn(_, _, _),
        );
        class.add_method(sel!(scrollWheel:), scroll_wheel as extern "C" fn(_, _, _));
        class.add_method(
            sel!(shouldDelayWindowOrderingForEvent:),
            should_delay_window_ordering as extern "C" fn(_, _, _) -> _,
        )
    }
}

fn with_state<F, FR, R>(this: &NSView, callback: F, default: FR) -> R
where
    F: FnOnce(Rc<PlatformDragContext>) -> R,
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

extern "C" fn mouse_down_can_move_window(_this: &NSView, _sel: Sel) -> Bool {
    Bool::YES
}

extern "C" fn mouse_down(this: &NSView, _sel: Sel, event: &NSEvent) {
    with_state(this, |state| state.on_mouse_down(event.retain()), || ());

    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), mouseDown: event];
    }
}

extern "C" fn mouse_up(this: &NSView, _sel: Sel, event: &NSEvent) {
    with_state(this, |state| state.on_mouse_up(event.retain()), || ());

    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), mouseUp: event];
    }
}

extern "C" fn right_mouse_down(this: &NSView, _sel: Sel, event: &NSEvent) {
    with_state(
        this,
        |state| state.on_right_mouse_down(event.retain()),
        || (),
    );

    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), rightMouseDown: event];
    }
}

extern "C" fn right_mouse_up(this: &NSView, _sel: Sel, event: &NSEvent) {
    with_state(this, |state| state.on_right_mouse_up(event.retain()), || ());

    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), rightMouseUp: event];
    }
}

extern "C" fn scroll_wheel(this: &NSView, _sel: Sel, event: &NSEvent) {
    if !with_state(this, |state| state.on_scroll_event(event.retain()), || true) {
        return;
    }
    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), scrollWheel: event];
    }
}

extern "C" fn source_operation_mask_for_dragging_context(
    this: &NSView,
    _: Sel,
    session: &NSDraggingSession,
    context: NSDraggingContext,
) -> NSDragOperation {
    with_state(
        this,
        move |state| state.source_operation_mask_for_dragging_context(session, context),
        || NSDragOperation::None,
    )
}

extern "C" fn dragging_session_ended_at_point(
    this: &NSView,
    _: Sel,
    session: &NSDraggingSession,
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
    this: &NSView,
    _: Sel,
    session: &NSDraggingSession,
    point: NSPoint,
) {
    with_state(this, move |state| state.drag_moved(session, point), || ())
}

extern "C" fn should_delay_window_ordering(this: &NSView, _: Sel, event: &NSEvent) -> Bool {
    with_state(
        this,
        move |state| {
            if state.should_delay_window_ordering(event) {
                Bool::YES
            } else {
                Bool::NO
            }
        },
        || Bool::YES,
    )
}
