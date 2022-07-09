use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
    sync::Arc,
};

use jni::{objects::JObject, sys::jsize, JNIEnv};
use log::info;

use crate::{
    android::{DRAG_DROP_UTIL, JAVA_VM},
    api_model::ImageData,
    drag_manager::{DragRequest, PlatformDragContextDelegate},
    error::{NativeExtensionsError, NativeExtensionsResult},
    util::DropNotifier,
};

use super::PlatformDataSource;

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
        writer: Rc<PlatformDataSource>,
        drop_notifier: Arc<DropNotifier>,
    ) -> NativeExtensionsResult<()> {
        let env = JAVA_VM
            .get()
            .ok_or_else(|| NativeExtensionsError::OtherError("JAVA_VM not set".into()))?
            .attach_current_thread()?;

        // TODO (Actually bind to drag session)
        thread_local! {
            static CURRENT_CLIP: RefCell<Arc<DropNotifier>> = RefCell::new(DropNotifier::new(||{}));
        }
        CURRENT_CLIP.with(|r| r.replace(drop_notifier));

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
