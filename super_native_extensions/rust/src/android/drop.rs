use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    sync::Arc,
};

use jni::{
    objects::{GlobalRef, JClass, JObject, JValue},
    sys::{jlong, jvalue},
    JNIEnv,
};

use nativeshell_core::{Context, Value};

use crate::{
    android::{DRAG_DROP_UTIL, JAVA_VM},
    api_model::{DropOperation, Point},
    drop_manager::{BaseDropEvent, DropEvent, PlatformDropContextDelegate},
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    reader_manager::RegisteredDataReader,
    util::{DropNotifier, NextId},
};

use super::{
    drag_common::{DragAction, DragEvent},
    PlatformDataReader,
};

pub struct PlatformDropContext {
    id: i64,
    view_handle: i64,
    delegate: Weak<dyn PlatformDropContextDelegate>,
    next_session: Cell<i64>,
    current_session: RefCell<Option<Rc<Session>>>,
}

struct Session {
    id: i64,
    last_operation: Cell<DropOperation>,
}

thread_local! {
    static CONTEXTS: RefCell<HashMap<i64, Weak<PlatformDropContext>>> = RefCell::new(HashMap::new());
}

impl PlatformDropContext {
    pub fn new(id: i64, view_handle: i64, delegate: Weak<dyn PlatformDropContextDelegate>) -> Self {
        Self {
            id,
            view_handle,
            delegate,
            next_session: Cell::new(0),
            current_session: RefCell::new(None),
        }
    }

    fn _assign_weak_self(&self, weak_self: Weak<Self>) -> NativeExtensionsResult<()> {
        CONTEXTS.with(|c| c.borrow_mut().insert(self.id, weak_self));

        let env = JAVA_VM
            .get()
            .ok_or_else(|| NativeExtensionsError::OtherError("JAVA_VM not set".into()))?
            .attach_current_thread()?;

        env.call_method(
            DRAG_DROP_UTIL.get().unwrap().as_obj(),
            "registerDropHandler",
            "(JJ)V",
            &[self.view_handle.into(), self.id.into()],
        )?;
        Ok(())
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self._assign_weak_self(weak_self).ok_log();
    }

    pub fn register_drop_types(&self, _types: &[String]) -> NativeExtensionsResult<()> {
        Ok(())
    }

    fn translate_drop_event<'a>(
        event: DragEvent<'a>,
        session_id: i64,
        env: &JNIEnv<'a>,
        local_data: Value,
        accepted_operation: Option<DropOperation>,
        reader: Option<RegisteredDataReader>,
    ) -> NativeExtensionsResult<DropEvent> {
        let clip_description = event.get_clip_description(env)?;
        let mime_type_count = env
            .call_method(clip_description, "getMimeTypeCount", "()I", &[])?
            .i()?;
        let mut mime_types = Vec::<String>::new();
        for i in 0..mime_type_count {
            let mime_type = env
                .call_method(
                    clip_description,
                    "getMimeType",
                    "(I)Ljava/lang/String;",
                    &[i.into()],
                )?
                .l()?;
            let mime_type = env.get_string(mime_type.into())?;
            mime_types.push(mime_type.into());
        }
        Ok(DropEvent {
            session_id,
            location_in_view: Point {
                x: event.get_x(env)? as f64,
                y: event.get_y(env)? as f64,
            },
            local_data,
            allowed_operations: vec![DropOperation::Copy],
            formats: mime_types,
            accepted_operation,
            reader,
        })
    }

    fn release_permissions(permissions: GlobalRef) -> NativeExtensionsResult<()> {
        let env = JAVA_VM
            .get()
            .ok_or_else(|| NativeExtensionsError::OtherError("JAVA_VM not set".into()))?
            .attach_current_thread()?;
        let permissions = permissions.as_obj();
        env.call_method(permissions, "release", "()V", &[])?;
        Ok(())
    }

    /// Request drag and drop permissions for the event. The permissions will
    /// be released when the drop notifier is droppped
    fn request_drag_drop_permissions<'a>(
        &self,
        env: &JNIEnv<'a>,
        event: JObject<'a>,
    ) -> NativeExtensionsResult<Arc<DropNotifier>> {
        let activity = env
            .call_method(
                DRAG_DROP_UTIL.get().unwrap().as_obj(),
                "getActivity",
                "(J)Landroid/app/Activity;",
                &[self.view_handle.into()],
            )?
            .l()?;
        let permission = env
            .call_method(
                activity,
                "requestDragAndDropPermissions",
                "(Landroid/view/DragEvent;)Landroid/view/DragAndDropPermissions;",
                &[event.into()],
            )?
            .l()?;
        let permissions = env.new_global_ref(permission)?;
        Ok(DropNotifier::new(move || {
            Self::release_permissions(permissions).ok_log();
        }))
    }

    fn on_drag_event<'a>(
        &self,
        env: &JNIEnv<'a>,
        event: JObject<'a>,
    ) -> NativeExtensionsResult<bool> {
        let event = DragEvent(event);
        if let Some(delegate) = self.delegate.upgrade() {
            // We're conflating drag and drop context ids here. However it works
            // because at this point there are both IsolateId. In future with
            // flutter multiview they should probably be based in view handle
            let drag_context = delegate
                .get_platform_drag_context(self.id)
                .expect("Missing drag context");
            // forward the event to drag context. Necessary to know when current
            // drag session ends for example.
            drag_context.on_drop_event(env, event)?;

            let current_session = {
                let mut session = self.current_session.borrow_mut();
                session
                    .get_or_insert_with(|| {
                        let id = self.next_session.next_id();
                        Rc::new(Session {
                            id,
                            last_operation: Cell::new(DropOperation::None),
                        })
                    })
                    .clone()
            };

            let action = event.get_action(env)?;
            match action {
                DragAction::DragLocation => {
                    let local_data = drag_context.get_local_data(env, event)?;
                    let event = Self::translate_drop_event(
                        event,
                        current_session.id,
                        env,
                        local_data,
                        None, // accepted operation
                        None, // reader
                    )?;
                    let session_clone = current_session.clone();
                    delegate.send_drop_update(
                        self.id,
                        event,
                        Box::new(move |res| {
                            session_clone
                                .last_operation
                                .replace(res.ok_log().unwrap_or(DropOperation::None));
                        }),
                    );
                    Ok(true)
                }
                DragAction::DragExited => {
                    delegate.send_drop_leave(
                        self.id,
                        BaseDropEvent {
                            session_id: current_session.id,
                        },
                    );
                    Ok(true)
                }
                DragAction::Drop => {
                    let accepted_operation = current_session.last_operation.get();
                    if accepted_operation != DropOperation::None
                        && accepted_operation != DropOperation::UserCancelled
                        && accepted_operation != DropOperation::Forbidden
                    {
                        let local_data = drag_context.get_local_data(env, event)?;
                        let clip_data = event.get_clip_data(env)?;
                        // If this is local data make sure to extend the lifetime
                        // with the reader.
                        let source_data_notifier = drag_context
                            .get_data_source_drop_notifier(env, event)?
                            .unwrap_or_else(|| DropNotifier::new(|| {}));

                        let permission_notifier =
                            self.request_drag_drop_permissions(env, event.0)?;

                        let reader = PlatformDataReader::from_clip_data(
                            env,
                            clip_data,
                            Some(DropNotifier::new_combined(&[
                                source_data_notifier,
                                permission_notifier,
                            ])),
                        )?;
                        let reader = delegate.register_platform_reader(reader)?;
                        let event = Self::translate_drop_event(
                            event,
                            current_session.id,
                            env,
                            local_data,
                            Some(accepted_operation),
                            Some(reader),
                        )?;
                        let done = Rc::new(Cell::new(false));
                        let done_clone = done.clone();
                        delegate.send_perform_drop(
                            self.id,
                            event,
                            Box::new(move |r| {
                                r.ok_log();
                                done_clone.set(true);
                            }),
                        );
                        while !done.get() {
                            Context::get().run_loop().platform_run_loop.poll_once();
                        }
                        return Ok(true);
                    } else {
                        Ok(false)
                    }
                }
                DragAction::DragEnded => {
                    delegate.send_drop_ended(
                        self.id,
                        BaseDropEvent {
                            session_id: current_session.id,
                        },
                    );
                    self.current_session.replace(None);
                    Ok(true)
                }
                _ => Ok(true),
            }
        } else {
            Ok(false)
        }
    }
}

impl Drop for PlatformDropContext {
    fn drop(&mut self) {
        CONTEXTS.with(|c| c.borrow_mut().remove(&self.id));
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn Java_com_superlist_super_1native_1extensions_DragDropUtil_onDrag(
    env: JNIEnv,
    _class: JClass,
    event: JObject,
    drag_context: jlong,
) -> jvalue {
    let context = CONTEXTS
        .with(|c| c.borrow().get(&drag_context).cloned())
        .and_then(|v| v.upgrade());
    match context {
        Some(context) => {
            let res = context.on_drag_event(&env, event).ok_log().unwrap_or(false);
            JValue::from(res).into()
        }
        None => JValue::from(false).into(),
    }
}
