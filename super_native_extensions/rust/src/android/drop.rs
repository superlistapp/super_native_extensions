use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use jni::{
    objects::{JClass, JObject, JValue},
    sys::{jint, jlong, jobject, jsize, jvalue},
    JNIEnv,
};
use log::info;

use crate::{
    android::{DRAG_DROP_UTIL, JAVA_VM},
    api_model::ImageData,
    drop_manager::PlatformDropContextDelegate,
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    platform,
};

pub struct PlatformDropContext {
    id: i64,
    view_handle: i64,
    delegate: Weak<dyn PlatformDropContextDelegate>,
}

thread_local! {
    static CONTEXTS: RefCell<HashMap<i64, Weak<PlatformDropContext>>> = RefCell::new(HashMap::new());
}

impl PlatformDropContext {
    pub fn new(id: i64, view_handle: i64, delegate: Weak<dyn PlatformDropContextDelegate>) -> Self {
        info!("VIEW {:?}", view_handle);
        Self {
            id,
            view_handle,
            delegate,
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

    pub fn register_drop_types(&self, types: &[String]) -> NativeExtensionsResult<()> {
        Ok(())
    }

    fn on_drag_event<'a>(
        &self,
        env: &JNIEnv<'a>,
        event: JObject<'a>,
    ) -> NativeExtensionsResult<bool> {
        Ok(true)
    }
}

impl Drop for PlatformDropContext {
    fn drop(&mut self) {
        CONTEXTS.with(|c| c.borrow_mut().remove(&self.id));
    }
}

fn get_flutter_view<'a>(
    env: &JNIEnv<'a>,
    binding: JObject<'a>,
) -> NativeExtensionsResult<JObject<'a>> {
    let engine = env
        .call_method(
            binding,
            "getFlutterEngine",
            "()Lio/flutter/embedding/engine/FlutterEngine;",
            &[],
        )?
        .l()?;
    let platform_views_controller = env
        .call_method(
            engine,
            "getPlatformViewsController",
            "()Lio/flutter/plugin/platform/PlatformViewsController;",
            &[],
        )?
        .l()?;
    let view = env
        .get_field(
            platform_views_controller,
            "flutterView",
            "Lio/flutter/embedding/android/FlutterView;",
        )?
        .l()?;
    Ok(view)
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn Java_com_superlist_super_1native_1extensions_DragDropUtil_getFlutterView(
    env: JNIEnv,
    _class: JClass,
    binding: JObject,
) -> jobject {
    let res = get_flutter_view(&env, binding);
    match res {
        Ok(value) => value.into_inner(),
        Err(_) => JObject::null().into_inner(),
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
            let res = context.on_drag_event(&env, event).unwrap_or(false);
            JValue::from(res).into()
        }
        None => JValue::from(false).into(),
    }
}
