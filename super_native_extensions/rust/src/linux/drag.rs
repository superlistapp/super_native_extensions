use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    os::raw::c_ulong,
    rc::{Rc, Weak},
    time::Duration,
};

use gdk::{
    glib::{translate::from_glib_none, WeakRef},
    prelude::StaticType,
    traits::{DeviceExt, SeatExt},
    Display, DragAction, DragCancelReason, DragContext, Event,
};

use gtk::{prelude::DragContextExtManual, traits::WidgetExt, SelectionData, Widget};
use gtk_sys::GtkWidget;
use irondash_engine_context::EngineContext;
use irondash_message_channel::{Late, Value};
use irondash_run_loop::RunLoop;

use crate::{
    api_model::{DataProviderId, DragConfiguration, DragRequest, DropOperation, Point},
    drag_manager::{
        DataProviderEntry, DragSessionId, PlatformDragContextDelegate, PlatformDragContextId,
    },
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    platform_impl::platform::drag_common::DropOperationExt,
    shadow::WithShadow,
};

use super::{
    common::{surface_from_image_data, synthesize_button_up},
    signal::Signal,
    DataObject,
};

pub struct PlatformDragContext {
    id: PlatformDragContextId,
    delegate: Weak<dyn PlatformDragContextDelegate>,
    weak_self: Late<Weak<Self>>,
    pub(crate) view: WeakRef<Widget>,
    button_press_hook: Late<c_ulong>,
    pub(crate) last_button_press_event: RefCell<Option<Event>>,
    sessions: RefCell<HashMap<DragContext, Rc<Session>>>,
}

struct Session {
    id: DragSessionId,
    context_id: PlatformDragContextId,
    context_delegate: Weak<dyn PlatformDragContextDelegate>,
    data_object: Rc<DataObject>,
    configuration: DragConfiguration,
    weak_self: Late<Weak<Self>>,
    last_position: RefCell<Point>,
    last_operation: Cell<DropOperation>,
}

impl Session {
    fn new(
        id: DragSessionId,
        context_id: PlatformDragContextId,
        context_delegate: Weak<dyn PlatformDragContextDelegate>,
        data_object: Rc<DataObject>,
        configuration: DragConfiguration,
    ) -> Rc<Self> {
        let res = Rc::new(Self {
            id,
            context_id,
            context_delegate,
            data_object,
            configuration,
            weak_self: Late::new(),
            last_position: RefCell::new(Point::default()),
            last_operation: Cell::new(DropOperation::None),
        });
        res.weak_self.set(Rc::downgrade(&res));
        res.schedule_update_position();
        res
    }

    fn schedule_update_position(&self) {
        let weak_self = self.weak_self.clone();
        RunLoop::current()
            .schedule(Duration::from_secs_f64(1.0 / 60.0), move || {
                if let Some(this) = weak_self.upgrade() {
                    this.update_position();
                }
            })
            .detach();
    }

    fn update_position(&self) {
        if let Some(display) = Display::default() {
            if let Some(seat) = display.default_seat() {
                if let Some(pointer) = seat.pointer() {
                    let position = pointer.position_double();
                    let position = Point {
                        x: position.1,
                        y: position.2,
                    };
                    let mut last_position = self.last_position.borrow_mut();
                    if *last_position != position {
                        *last_position = position.clone();
                        if let Some(delegate) = self.context_delegate.upgrade() {
                            delegate.drag_session_did_move_to_location(
                                self.context_id,
                                self.id,
                                position,
                            );
                        }
                    }
                }
            }
        }
        self.schedule_update_position();
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(delegate) = self.context_delegate.upgrade() {
            delegate.drag_session_did_end_with_operation(
                self.context_id,
                self.id,
                self.last_operation.get(),
            );
        }
    }
}

impl PlatformDragContext {
    pub fn new(
        id: PlatformDragContextId,
        engine_handle: i64,
        delegate: Weak<dyn PlatformDragContextDelegate>,
    ) -> NativeExtensionsResult<Self> {
        unsafe { gtk::set_initialized() };

        let view = EngineContext::get()?.get_flutter_view(engine_handle)?;
        let view: Widget = unsafe { from_glib_none(view as *mut GtkWidget) };
        let weak = WeakRef::new();
        weak.set(Some(&view));

        Ok(Self {
            id,
            weak_self: Late::new(),
            view: weak,
            button_press_hook: Late::new(),
            delegate,
            last_button_press_event: RefCell::new(None),
            sessions: RefCell::new(HashMap::new()),
        })
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self.clone());
        if let Some(signal) = Signal::lookup("button-press-event", Widget::static_type()) {
            let hook = signal.add_emission_hook(move |_, values| {
                if let Some(this) = weak_self.clone().upgrade() {
                    if let Some(event) = values[1].get::<Event>().ok_log() {
                        this.last_button_press_event.replace(Some(event));
                    }
                }
                true
            });
            self.button_press_hook.set(hook);
        }
        if let Some(view) = self.view.upgrade() {
            let weak_self = self.weak_self.clone();
            view.connect_drag_data_get(move |_, context, data, _target_info, _time| {
                if let Some(this) = weak_self.upgrade() {
                    this.get_data(context, data);
                }
            });
        }
    }

    pub fn get_data(&self, context: &DragContext, data: &SelectionData) {
        if let Some(session) = self.sessions.borrow().get(context).cloned() {
            session.data_object.get_data(data).ok_log();
        }
    }

    pub fn needs_combined_drag_image() -> bool {
        true
    }

    fn view(&self) -> NativeExtensionsResult<Widget> {
        self.view
            .upgrade()
            .ok_or_else(|| NativeExtensionsError::OtherError("View was already released".into()))
    }

    pub async fn start_drag(
        &self,
        request: DragRequest,
        mut providers: HashMap<DataProviderId, DataProviderEntry>,
        session_id: DragSessionId,
    ) -> NativeExtensionsResult<()> {
        let providers: Vec<_> = request
            .configuration
            .items
            .iter()
            .map(|item| {
                let entry = providers
                    .remove(&item.data_provider_id)
                    .expect("Missing data provider entry");
                (entry.provider, entry.handle)
            })
            .collect();
        let object = DataObject::new(providers);
        let target_list = object.create_target_list();
        let event = self
            .last_button_press_event
            .borrow()
            .as_ref()
            .cloned()
            .ok_or_else(|| NativeExtensionsError::OtherError("Missing mouse event".into()))?;

        // release event will get eaten
        let mut release = synthesize_button_up(&event);
        gtk::main_do_event(&mut release);

        let view = self.view()?;
        let mut actions = DragAction::empty();
        for operation in &request.configuration.allowed_operations {
            actions |= operation.to_platform();
        }
        let context = view.drag_begin_with_coordinates(
            &target_list,
            actions,
            event.button().unwrap_or(0) as i32,
            Some(&event),
            request.position.x as i32,
            request.position.y as i32,
        );
        if let Some(context) = context {
            if let Some(image) = request.combined_drag_image {
                let image = image.with_shadow(10);
                let scale = image.image_data.device_pixel_ratio.unwrap_or(1.0);
                let surface = surface_from_image_data(image.image_data, 0.8);
                surface.set_device_offset(
                    (image.rect.x - request.position.x) * scale,
                    (image.rect.y - request.position.y) * scale,
                );
                context.drag_set_icon_surface(&surface)
            }
            let session = Session::new(
                session_id,
                self.id,
                self.delegate.clone(),
                object,
                request.configuration,
            );
            self.sessions.borrow_mut().insert(context.clone(), session);
            let weak_self = self.weak_self.clone();
            context.connect_cancel(move |context, reason| {
                if let Some(this) = weak_self.upgrade() {
                    if let Some(session) = this.sessions.borrow_mut().remove(context) {
                        match reason {
                            DragCancelReason::UserCancelled => {
                                session.last_operation.replace(DropOperation::UserCancelled)
                            }
                            _ => session.last_operation.replace(DropOperation::None),
                        };
                    }
                }
            });
            let weak_self = self.weak_self.clone();
            context.connect_dnd_finished(move |context| {
                if let Some(this) = weak_self.upgrade() {
                    if let Some(session) = this.sessions.borrow_mut().remove(context) {
                        session
                            .last_operation
                            .replace(DropOperation::from_platform(context.selected_action()));
                    }
                }
            });
        }

        Ok(())
    }

    pub fn get_local_data(&self) -> Option<Vec<Value>> {
        self.sessions
            .borrow()
            .iter()
            .next()
            .map(|a| a.1.clone())
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
                if s.1.id == session_id {
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
        if let Some(signal) = Signal::lookup("button-press-event", Widget::static_type()) {
            signal.remove_emission_hook(*self.button_press_hook);
        }
    }
}
