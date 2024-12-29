use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    sync::Arc,
    thread,
};

use irondash_engine_context::EngineContext;
use irondash_message_channel::{IsolateId, Value};
use jni::{
    objects::{GlobalRef, JClass, JObject, JString, JValue},
    sys::{jint, jlong, jvalue},
    JNIEnv,
};

use crate::{
    android::{CONTEXT, DRAG_DROP_HELPER, JAVA_VM},
    api_model::{DropOperation, Point},
    drop_manager::{
        BaseDropEvent, DropEvent, DropItem, DropSessionId, PlatformDropContextDelegate,
        PlatformDropContextId,
    },
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    reader_manager::RegisteredDataReader,
    util::{DropNotifier, NextId},
    value_promise::Promise,
};

use super::{
    drag_common::{DragAction, DragEvent},
    PlatformDataReader,
};

pub struct PlatformDropContext {
    id: PlatformDropContextId,
    engine_handle: i64,
    delegate: Weak<dyn PlatformDropContextDelegate>,
    next_session_id: Cell<i64>,
    current_session: RefCell<Option<Rc<Session>>>,
}

struct Session {
    id: DropSessionId,
    last_operation: Cell<DropOperation>,
}

thread_local! {
    static CONTEXTS: RefCell<HashMap<PlatformDropContextId, Weak<PlatformDropContext>>> = RefCell::new(HashMap::new());
}

impl PlatformDropContext {
    pub fn new(
        id: PlatformDropContextId,
        engine_handle: i64,
        delegate: Weak<dyn PlatformDropContextDelegate>,
    ) -> NativeExtensionsResult<Self> {
        Ok(Self {
            id,
            engine_handle,
            delegate,
            next_session_id: Cell::new(0),
            current_session: RefCell::new(None),
        })
    }

    fn _assign_weak_self(&self, weak_self: Weak<Self>) -> NativeExtensionsResult<()> {
        CONTEXTS.with(|c| c.borrow_mut().insert(self.id, weak_self));

        let mut env = JAVA_VM
            .get()
            .ok_or_else(|| NativeExtensionsError::OtherError("JAVA_VM not set".into()))?
            .attach_current_thread()?;

        let view = EngineContext::get()?.get_flutter_view(self.engine_handle)?;

        env.call_method(
            DRAG_DROP_HELPER.get().unwrap().as_obj(),
            "registerDropHandler",
            "(Landroid/view/View;J)V",
            &[view.as_obj().into(), self.id.0.into()],
        )?;
        Ok(())
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self._assign_weak_self(weak_self).ok_log();
    }

    pub fn register_drop_formats(&self, _formats: &[String]) -> NativeExtensionsResult<()> {
        Ok(())
    }

    fn get_display_density(env: &mut JNIEnv) -> NativeExtensionsResult<f64> {
        let context = CONTEXT.get().unwrap().as_obj();
        let resources = env
            .call_method(
                context,
                "getResources",
                "()Landroid/content/res/Resources;",
                &[],
            )?
            .l()?;
        let display_metrics = env
            .call_method(
                resources,
                "getDisplayMetrics",
                "()Landroid/util/DisplayMetrics;",
                &[],
            )?
            .l()?;
        let density = env.get_field(display_metrics, "density", "F")?.f()?;
        Ok(density as f64)
    }

    fn translate_drop_event<'a>(
        event: &DragEvent<'a, '_>,
        session_id: DropSessionId,
        env: &mut JNIEnv<'a>,
        mut local_data: Vec<Value>,
        allowed_operations: Vec<DropOperation>,
        accepted_operation: Option<DropOperation>,
        reader: Option<(Rc<PlatformDataReader>, RegisteredDataReader)>,
    ) -> NativeExtensionsResult<DropEvent> {
        let items = match reader.as_ref() {
            Some((reader, _)) => {
                // we have access to actual clip data so use it to build items
                let mut items = Vec::new();
                for (index, item) in reader.get_items_sync()?.iter().enumerate() {
                    items.push(DropItem {
                        item_id: (index as i64).into(),
                        formats: reader.get_formats_for_item_sync(*item)?,
                        local_data: local_data.get(index).cloned().unwrap_or(Value::Null),
                    });
                }
                items
            }
            None => {
                // here we only have clip description; The number of reported data will
                // be number or local items (if any), or 1. Each item will have types
                // from clip description set.
                let clip_description = event.get_clip_description(env)?;
                let mime_types = if env.is_same_object(&clip_description, JObject::null())? {
                    Vec::default()
                } else {
                    let mime_type_count = env
                        .call_method(&clip_description, "getMimeTypeCount", "()I", &[])?
                        .i()?;
                    let mut mime_types = Vec::<String>::new();
                    for i in 0..mime_type_count {
                        let mime_type: JString = env
                            .call_method(
                                &clip_description,
                                "getMimeType",
                                "(I)Ljava/lang/String;",
                                &[i.into()],
                            )?
                            .l()?
                            .into();
                        let mime_type = env.get_string(&mime_type)?;
                        mime_types.push(mime_type.into());
                    }
                    mime_types
                };

                if local_data.is_empty() {
                    local_data.push(Value::Null);
                }
                local_data
                    .into_iter()
                    .enumerate()
                    .map(|(index, local_data)| DropItem {
                        item_id: (index as i64).into(),
                        formats: mime_types.clone(),
                        local_data,
                    })
                    .collect()
            }
        };

        let density = Self::get_display_density(env)?;
        Ok(DropEvent {
            session_id,
            location_in_view: Point {
                x: event.get_x(env)? as f64 / density,
                y: event.get_y(env)? as f64 / density,
            },
            allowed_operations,
            items,
            accepted_operation,
            reader: reader.map(|r| r.1),
        })
    }

    fn release_permissions(permissions: GlobalRef) -> NativeExtensionsResult<()> {
        let mut env = JAVA_VM
            .get()
            .ok_or_else(|| NativeExtensionsError::OtherError("JAVA_VM not set".into()))?
            .attach_current_thread()?;
        let permissions = permissions.as_obj();
        env.call_method(permissions, "release", "()V", &[])?;
        Ok(())
    }

    /// Request drag and drop permissions for the event. The permissions will
    /// be released when the drop notifier is dropped
    fn request_drag_drop_permissions<'a>(
        &self,
        env: &mut JNIEnv<'a>,
        event: &JObject<'a>,
    ) -> NativeExtensionsResult<Arc<DropNotifier>> {
        let activity = EngineContext::get()?.get_activity(self.engine_handle)?;
        let permission = env
            .call_method(
                activity.as_obj(),
                "requestDragAndDropPermissions",
                "(Landroid/view/DragEvent;)Landroid/view/DragAndDropPermissions;",
                &[(&event).into()],
            )?
            .l()?;
        let permissions = env.new_global_ref(permission)?;
        Ok(Arc::new(DropNotifier::new(move || {
            Self::release_permissions(permissions).ok_log();
        })))
    }

    fn on_drag_event<'a>(
        &self,
        env: &mut JNIEnv<'a>,
        event: &JObject<'a>,
    ) -> NativeExtensionsResult<bool> {
        let event = DragEvent(event);
        if let Some(delegate) = self.delegate.upgrade() {
            // We're conflating drag and drop context ids here. However it works
            // because at this point there are both IsolateId. In future with
            // flutter multi-view they should probably be based in view handle
            let drag_contexts = delegate.get_platform_drag_contexts();

            for drag_context in &drag_contexts {
                // forward the event to drag context. Necessary to know when current
                // drag session ends for example.
                drag_context.on_drop_event(env, &event)?;
            }

            let current_session = {
                let mut session = self.current_session.borrow_mut();
                session
                    .get_or_insert_with(|| {
                        let id = self.next_session_id.next_id();
                        Rc::new(Session {
                            id: id.into(),
                            last_operation: Cell::new(DropOperation::None),
                        })
                    })
                    .clone()
            };

            let session_id = event.get_session_id(env)?;

            let get_local_data = || {
                session_id
                    .and_then(|session_id| {
                        drag_contexts
                            .iter()
                            .filter_map(|c| c.get_local_data_for_session_id(session_id).ok())
                            .next()
                    })
                    .unwrap_or_default()
            };

            let get_allowed_operations = || {
                session_id
                    .and_then(|session_id| {
                        drag_contexts
                            .iter()
                            .filter_map(|c| c.get_allowed_operations(session_id))
                            .next()
                    })
                    .unwrap_or_else(|| vec![DropOperation::Copy])
            };

            let get_data_provider_handles = || {
                session_id
                    .and_then(|session_id| {
                        drag_contexts
                            .iter()
                            .filter_map(|c| c.get_data_provider_handles(session_id))
                            .next()
                    })
                    .unwrap_or_default()
            };

            let action = event.get_action(env)?;
            match action {
                DragAction::DragLocation => {
                    let event = Self::translate_drop_event(
                        &event,
                        current_session.id,
                        env,
                        get_local_data(),
                        get_allowed_operations(),
                        None, // accepted operation
                        None, // reader
                    )?;
                    let weak_delegate = self.delegate.clone();
                    delegate.send_drop_update(
                        self.id,
                        event,
                        Box::new(move |res| {
                            let operation = res.ok_log().unwrap_or(DropOperation::None);
                            if let (Some(session_id), Some(delegate)) =
                                (session_id, weak_delegate.upgrade())
                            {
                                delegate
                                    .get_platform_drag_contexts()
                                    .iter()
                                    .for_each(|d| d.replace_last_operation(session_id, operation));
                            }
                            current_session.last_operation.replace(operation);
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
                        let local_data = get_local_data();
                        let clip_data = event.get_clip_data(env)?;

                        let reader = if env.is_same_object(&clip_data, JObject::null())? {
                            None
                        } else {
                            // If this is local data make sure to extend the lifetime
                            // with the reader.
                            let data_provider_handles = get_data_provider_handles();

                            let permission_notifier =
                                self.request_drag_drop_permissions(env, event.0)?;

                            let reader = PlatformDataReader::from_clip_data(
                                env,
                                clip_data,
                                Some(Arc::new(DropNotifier::new(move || {
                                    let _data_provider_handles = data_provider_handles;
                                    let _permission_notifier = permission_notifier;
                                }))),
                            )?;
                            let registered_reader =
                                delegate.register_platform_reader(self.id, reader.clone());
                            Some((reader, registered_reader))
                        };

                        let event = Self::translate_drop_event(
                            &event,
                            current_session.id,
                            env,
                            local_data,
                            get_allowed_operations(),
                            Some(accepted_operation),
                            reader,
                        )?;
                        delegate.send_perform_drop(
                            self.id,
                            event,
                            Box::new(move |r| {
                                r.ok_log();
                            }),
                        );
                        Ok(true)
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
        CONTEXTS.try_with(|c| c.borrow_mut().remove(&self.id)).ok();
    }
}

fn update_last_touch_point<'a>(
    env: &mut JNIEnv<'a>,
    view_root: JObject<'a>,
    x: i32,
    y: i32,
) -> NativeExtensionsResult<()> {
    let view_root_global = env.new_global_ref(&view_root)?;
    let jvm = env.get_java_vm()?;
    let p = Arc::new(Promise::new());
    let p2 = p.clone();
    thread::spawn(move || {
        let update = move || -> NativeExtensionsResult<()> {
            let mut env = jvm.attach_current_thread()?;
            let view_root = view_root_global.as_obj();
            let last_touch_point = env
                .get_field(view_root, "mLastTouchPoint", "Landroid/graphics/PointF;")?
                .l()?;
            env.set_field(&last_touch_point, "x", "F", (x as f32).into())?;
            env.set_field(&last_touch_point, "y", "F", (y as f32).into())?;
            Ok(())
        };
        p.set(update());
    });
    p2.wait()?;

    Ok(())
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn Java_com_superlist_super_1native_1extensions_DragDropHelper_updateLastTouchPoint<
    'a,
>(
    mut env: JNIEnv<'a>,
    _class: JClass,
    view_root: JObject<'a>,
    x: jint,
    y: jint,
) {
    update_last_touch_point(&mut env, view_root, x, y).ok_log();
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn Java_com_superlist_super_1native_1extensions_DragDropHelper_onDrag<'a>(
    mut env: JNIEnv<'a>,
    _class: JClass,
    event: JObject<'a>,
    drag_context: jlong,
) -> jvalue {
    let context = CONTEXTS
        .with(|c| c.borrow().get(&IsolateId(drag_context)).cloned())
        .and_then(|v| v.upgrade());
    match context {
        Some(context) => {
            let res = context
                .on_drag_event(&mut env, &event)
                .ok_log()
                .unwrap_or(false);
            JValue::from(res).as_jni()
        }
        None => JValue::from(false).as_jni(),
    }
}
