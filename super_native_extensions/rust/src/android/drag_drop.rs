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
    drag_drop_manager::{DragRequest, PlatformDragContextDelegate},
    error::{NativeExtensionsError, NativeExtensionsResult},
    platform,
};

use super::PlatformClipboardWriter;

pub struct PlatformDragContext {
    id: i64,
    view_handle: i64,
    delegate: Weak<dyn PlatformDragContextDelegate>,
}

thread_local! {
    static CONTEXTS: RefCell<HashMap<i64, Weak<PlatformDragContext>>> = RefCell::new(HashMap::new());
}

impl PlatformDragContext {
    pub fn new(id: i64, view_handle: i64, delegate: Weak<dyn PlatformDragContextDelegate>) -> Self {
        info!("VIEW {:?}", view_handle);
        Self {
            id,
            view_handle,
            delegate,
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) -> NativeExtensionsResult<()> {
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

    pub fn register_drop_types(&self, types: &[String]) -> NativeExtensionsResult<()> {
        Ok(())
    }

    fn create_bitmap<'a>(
        env: &JNIEnv<'a>,
        image: &ImageData,
    ) -> NativeExtensionsResult<JObject<'a>> {
        let mut tmp = Vec::<i32>::new();
        tmp.resize((image.width * image.height) as usize, 0);

        for y in 0..image.height as usize {
            for x in 0..image.width as usize {
                let pos = y * image.bytes_per_row as usize + x * 4;
                let r = image.data[pos] as i32;
                let g = image.data[pos + 1] as i32;
                let b = image.data[pos + 2] as i32;
                let a = image.data[pos + 3] as i32;
                // Contrary to what ARGB_8888 documentation says the colors are
                // indeed encoded in ARGB order.
                let color = (a & 0xff) << 24 | (r & 0xff) << 16 | (g & 0xff) << 8 | (b & 0xff);
                tmp[y * image.width as usize + x] = color;
            }
        }

        let colors = env.new_int_array(tmp.len() as jsize)?;
        env.set_int_array_region(colors, 0, &tmp)?;
        info!("A");
        let config = env
            .call_static_method(
                "android/graphics/Bitmap$Config",
                "valueOf",
                "(Ljava/lang/String;)Landroid/graphics/Bitmap$Config;",
                &[env.new_string("ARGB_8888")?.into()],
            )?
            .l()?;

        info!("B");

        let res = env
            .call_static_method(
                "android/graphics/Bitmap",
                "createBitmap",
                "([IIIIILandroid/graphics/Bitmap$Config;)Landroid/graphics/Bitmap;",
                &[
                    colors.into(),
                    0.into(),           // offset
                    image.width.into(), // stride
                    image.width.into(),
                    image.height.into(),
                    config.into(),
                ],
            )?
            .l()?;
        Ok(res)
    }

    pub async fn start_drag(
        &self,
        request: DragRequest,
        writer: Rc<PlatformClipboardWriter>,
    ) -> NativeExtensionsResult<()> {
        let env = JAVA_VM
            .get()
            .ok_or_else(|| NativeExtensionsError::OtherError("JAVA_VM not set".into()))?
            .attach_current_thread()?;

        let data = writer.create_clip_data(&env)?;

        let bitmap = Self::create_bitmap(&env, &request.image)?;

        env.call_method(
            DRAG_DROP_UTIL.get().unwrap().as_obj(),
            "startDrag",
            "(JLandroid/content/ClipData;Landroid/graphics/Bitmap;II)V",
            &[
                self.view_handle.into(),
                data.into(),
                bitmap.into(),
                (request.point_in_rect.x.round() as i32).into(),
                (request.point_in_rect.y.round() as i32).into(),
            ],
        )?;

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

impl Drop for PlatformDragContext {
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
