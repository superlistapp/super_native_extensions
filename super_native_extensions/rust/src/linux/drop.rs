use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use gdk::{
    glib::{translate::from_glib_none, WeakRef},
    Atom, DragAction, DragContext,
};

use gtk::{
    prelude::{DragContextExtManual, WidgetExtManual},
    traits::WidgetExt,
    DestDefaults, TargetList, Widget,
};
use gtk_sys::GtkWidget;
use irondash_engine_context::EngineContext;
use irondash_message_channel::{Late, Value};
use irondash_run_loop::RunLoop;

use crate::{
    api_model::{DropOperation, Point},
    drop_manager::{
        BaseDropEvent, DropEvent, DropItem, DropSessionId, PlatformDropContextDelegate,
        PlatformDropContextId,
    },
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    reader_manager::RegisteredDataReader,
    util::{NextId, TryGetOrInsert},
};

use super::{
    common::{TargetListExt, TYPE_TEXT, TYPE_URI},
    drag_common::DropOperationExt,
    PlatformDataReader, WidgetReader,
};

pub struct PlatformDropContext {
    id: PlatformDropContextId,
    delegate: Weak<dyn PlatformDropContextDelegate>,
    weak_self: Late<Weak<Self>>,
    view: WeakRef<Widget>,
    next_session_id: Cell<i64>,
    current_session: RefCell<Option<Rc<Session>>>,
}

struct Session {
    id: DropSessionId,
    widget_reader: Rc<WidgetReader>,
    platform_reader: Rc<PlatformDataReader>,
    registered_reader: RegisteredDataReader,
    last_operation: Cell<DropOperation>,
}

impl PlatformDropContext {
    pub fn new(
        id: PlatformDropContextId,
        engine_handle: i64,
        delegate: Weak<dyn PlatformDropContextDelegate>,
    ) -> NativeExtensionsResult<Self> {
        unsafe { gtk::set_initialized() };

        let view = EngineContext::get()?.get_flutter_view(engine_handle)?;

        let view: Widget = unsafe { from_glib_none(view as *mut GtkWidget) };
        let weak = WeakRef::new();
        weak.set(Some(&view));

        Ok(Self {
            id,
            delegate,
            weak_self: Late::new(),
            view: weak,
            next_session_id: Cell::new(0),
            current_session: RefCell::new(None),
        })
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);

        if let Some(view) = self.view.upgrade() {
            let weak_self = self.weak_self.clone();
            view.connect_drag_motion(move |_, c, x, y, time| {
                if let Some(this) = weak_self.upgrade() {
                    this.drag_motion(c, x, y, time).ok_log().unwrap_or(false)
                } else {
                    false
                }
            });
            let weak_self = self.weak_self.clone();
            view.connect_drag_leave(move |_, c, time| {
                let c = c.clone();
                let weak_self = weak_self.clone();
                // Ensure drag_leave comes after drag drop
                RunLoop::current()
                    .schedule_next(move || {
                        if let Some(this) = weak_self.upgrade() {
                            this.drag_leave(&c, time).ok_log();
                        }
                    })
                    .detach();
            });
            let weak_self = self.weak_self.clone();
            view.connect_drag_drop(move |_, c, x, y, time| {
                if let Some(this) = weak_self.upgrade() {
                    this.drag_drop(c, x, y, time).ok_log().unwrap_or(false)
                } else {
                    false
                }
            });
        }
    }

    fn view(&self) -> NativeExtensionsResult<Widget> {
        self.view
            .upgrade()
            .ok_or_else(|| NativeExtensionsError::OtherError("View was already released".into()))
    }

    fn delegate(&self) -> NativeExtensionsResult<Rc<dyn PlatformDropContextDelegate>> {
        self.delegate
            .upgrade()
            .ok_or_else(|| NativeExtensionsError::OtherError("missing context delegate".into()))
    }

    fn new_session(&self, context: &DragContext) -> NativeExtensionsResult<Rc<Session>> {
        let widget_reader = WidgetReader::new(context.clone(), self.view()?);
        let platform_reader = PlatformDataReader::new_with_widget_reader(widget_reader.clone())?;
        let registered_reader = self
            .delegate()?
            .register_platform_reader(self.id, platform_reader.clone());

        Ok(Rc::new(Session {
            id: self.next_session_id.next_id().into(),
            widget_reader,
            platform_reader,
            registered_reader,
            last_operation: Cell::new(DropOperation::None),
        }))
    }

    fn create_drop_event(
        &self,
        session: &Rc<Session>,
        context: &DragContext,
        x: i32,
        y: i32,
        accepted_operation: Option<DropOperation>,
    ) -> Option<DropEvent> {
        let reader_info = session.platform_reader.reader_info()?;

        let local_data = self
            .delegate()
            .ok()?
            .get_platform_drag_contexts()
            .iter()
            .map(|c| c.get_local_data())
            .find(|c| c.is_some())
            .flatten()
            .unwrap_or_default();

        let number_of_items = local_data.len().max(reader_info.number_of_items);
        Some(DropEvent {
            session_id: session.id,
            location_in_view: Point {
                x: x as f64,
                y: y as f64,
            },
            allowed_operations: DropOperation::from_platform_mask(context.actions()),
            accepted_operation,
            items: (0..number_of_items)
                .map(|i| DropItem {
                    item_id: (i as i64).into(),
                    formats: reader_info
                        .targets
                        .iter()
                        .filter(|f| i == 0 || *f == TYPE_URI)
                        .cloned()
                        .collect(),
                    local_data: local_data.get(i).cloned().unwrap_or(Value::Null),
                })
                .collect(),
            reader: Some(session.registered_reader.clone()),
        })
    }

    fn drag_motion(
        &self,
        context: &DragContext,
        x: i32,
        y: i32,
        time: u32,
    ) -> NativeExtensionsResult<bool> {
        let session = self
            .current_session
            .borrow_mut()
            .try_get_or_insert_with(|| self.new_session(context))?
            .clone();
        session.widget_reader.update_current_time(time);
        if let Some(event) = self.create_drop_event(&session, context, x, y, None) {
            let session_clone = session.clone();
            let context = context.clone();
            self.delegate()?.send_drop_update(
                self.id,
                event,
                Box::new(move |res| {
                    let res = res.ok_log().unwrap_or(DropOperation::None);
                    session_clone.last_operation.set(res);
                    context.drag_status(res.to_platform(), time);
                }),
            );
        } else {
            context.drag_status(DragAction::empty(), time);
        }
        Ok(true)
    }

    fn drag_drop(
        &self,
        context: &DragContext,
        x: i32,
        y: i32,
        time: u32,
    ) -> NativeExtensionsResult<bool> {
        let session = self.current_session.borrow_mut().take();
        if let Some(session) = session {
            session.widget_reader.update_current_time(time);

            if let Some(event) =
                self.create_drop_event(&session, context, x, y, Some(session.last_operation.get()))
            {
                let done = Rc::new(Cell::new(Option::<bool>::None));
                let done_clone = done.clone();
                self.delegate()?.send_perform_drop(
                    self.id,
                    event,
                    Box::new(move |r| {
                        let ok = r.ok_log().is_some();
                        done_clone.set(Some(ok));
                    }),
                );
                while done.get().is_none() {
                    RunLoop::current().platform_run_loop.poll_once();
                }
                let context = context.clone();
                let deleting = session.last_operation.get() == DropOperation::Move;
                let ok = done.get().unwrap_or(false);
                session.widget_reader.on_all_requests_resolved(move || {
                    context.drag_finish(ok, deleting, time);
                });
                self.delegate()?.send_drop_ended(
                    self.id,
                    BaseDropEvent {
                        session_id: session.id,
                    },
                );
            } else {
                context.drag_finish(false, false, time);
            }
        } else {
            context.drag_finish(false, false, time);
        }
        Ok(true)
    }

    fn drag_leave(&self, _context: &DragContext, _time: u32) -> NativeExtensionsResult<()> {
        if let Some(session) = self.current_session.take() {
            self.delegate()?.send_drop_leave(
                self.id,
                BaseDropEvent {
                    session_id: session.id,
                },
            );
            self.delegate()?.send_drop_ended(
                self.id,
                BaseDropEvent {
                    session_id: session.id,
                },
            );
        }
        Ok(())
    }

    pub fn register_drop_formats(&self, formats: &[String]) -> NativeExtensionsResult<()> {
        let list = TargetList::new(&[]);
        for format in formats {
            if format == TYPE_TEXT {
                list.add_text_targets(0);
            } else {
                list.add(&Atom::intern(format), 0, 0);
            }
        }
        let entries = list.get_target_entries();
        self.view()?.drag_dest_set(
            // Gtk documentation says that when calling get_drag_data from drag_motion the
            // DestDefaults::DROP flag should be set, but that causes nautilus to lock up.
            // Not having the flag and calling drag_finish manually seems to work fine
            DestDefaults::empty(),
            &entries,
            DragAction::MOVE | DragAction::COPY | DragAction::LINK,
        );
        Ok(())
    }
}
