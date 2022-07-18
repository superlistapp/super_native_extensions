use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
    sync::Arc,
};

use jni::{objects::JObject, sys::jsize, JNIEnv};
use nativeshell_core::Value;

use crate::{
    android::{DRAG_DROP_UTIL, JAVA_VM},
    api_model::{DragRequest, DropOperation, ImageData, Point},
    drag_manager::{DragSessionId, PlatformDragContextDelegate, PlatformDragContextId},
    error::{NativeExtensionsError, NativeExtensionsResult},
    util::DropNotifier,
};

use super::{
    drag_common::{DragAction, DragEvent},
    PlatformDataSource,
};

pub struct PlatformDragContext {
    id: PlatformDragContextId,
    view_handle: i64,
    delegate: Weak<dyn PlatformDragContextDelegate>,
    sessions: RefCell<HashMap<DragSessionId, DragSession>>,
}

struct DragSession {
    data_source_notifier: Arc<DropNotifier>,
    local_data: Value,
    platform_context_id: PlatformDragContextId,
    platform_context_delegate: Weak<dyn PlatformDragContextDelegate>,
}

thread_local! {
    static CONTEXTS: RefCell<HashMap<i64, Weak<PlatformDragContext>>> = RefCell::new(HashMap::new());
}

impl PlatformDragContext {
    pub fn new(id: i64, view_handle: i64, delegate: Weak<dyn PlatformDragContextDelegate>) -> Self {
        Self {
            id,
            view_handle,
            delegate,
            sessions: RefCell::new(HashMap::new()),
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        CONTEXTS.with(|c| c.borrow_mut().insert(self.id, weak_self));
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
        let config = env
            .call_static_method(
                "android/graphics/Bitmap$Config",
                "valueOf",
                "(Ljava/lang/String;)Landroid/graphics/Bitmap$Config;",
                &[env.new_string("ARGB_8888")?.into()],
            )?
            .l()?;

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
        data_source: Rc<PlatformDataSource>,
        drop_notifier: Arc<DropNotifier>,
        session_id: DragSessionId,
    ) -> NativeExtensionsResult<()> {
        let env = JAVA_VM
            .get()
            .ok_or_else(|| NativeExtensionsError::OtherError("JAVA_VM not set".into()))?
            .attach_current_thread()?;

        let data = data_source.create_clip_data(&env)?;

        let bitmap = Self::create_bitmap(&env, &request.configuration.drag_image.image_data)?;
        let point_in_rect = request.configuration.drag_image.point_in_rect;

        let mut sessions = self.sessions.borrow_mut();
        sessions.insert(
            session_id,
            DragSession {
                data_source_notifier: drop_notifier,
                local_data: request.configuration.local_data,
                platform_context_id: self.id,
                platform_context_delegate: self.delegate.clone(),
            },
        );

        let session_id: i64 = session_id.into();
        env.call_method(
            DRAG_DROP_UTIL.get().unwrap().as_obj(),
            "startDrag",
            "(JJLandroid/content/ClipData;Landroid/graphics/Bitmap;II)V",
            &[
                self.view_handle.into(),
                session_id.into(),
                data.into(),
                bitmap.into(),
                (point_in_rect.x.round() as i32).into(),
                (point_in_rect.y.round() as i32).into(),
            ],
        )?;

        Ok(())
    }

    pub fn on_drop_event<'a>(
        &self,
        env: &JNIEnv<'a>,
        event: DragEvent<'a>,
    ) -> NativeExtensionsResult<()> {
        let session_id = event.get_session_id(env)?;
        if let Some(session_id) = session_id {
            let mut sessions = self.sessions.borrow_mut();
            if let Some(session) = sessions.get(&session_id) {
                if session.handle_event(session_id, env, event)? == HandleEventResult::RemoveSession
                {
                    sessions.remove(&session_id);
                }
            }
        }
        Ok(())
    }

    pub fn get_local_data<'a>(
        &self,
        env: &JNIEnv<'a>,
        event: DragEvent<'a>,
    ) -> NativeExtensionsResult<Value> {
        let session_id = event.get_session_id(env)?;
        match session_id {
            Some(session_id) => {
                let sessions = self.sessions.borrow();
                let session = sessions.get(&session_id);
                match session {
                    Some(session) => Ok(session.local_data.clone()),
                    None => Ok(Value::Null),
                }
            }
            None => Ok(Value::Null),
        }
    }

    pub fn get_data_source_drop_notifier<'a>(
        &self,
        env: &JNIEnv<'a>,
        event: DragEvent<'a>,
    ) -> NativeExtensionsResult<Option<Arc<DropNotifier>>> {
        let session_id = event.get_session_id(env)?;
        match session_id {
            Some(session_id) => {
                let sessions = self.sessions.borrow();
                let session = sessions.get(&session_id);
                match session {
                    Some(session) => Ok(Some(session.data_source_notifier.clone())),
                    None => Ok(None),
                }
            }
            None => Ok(None),
        }
    }
}

#[derive(PartialEq)]
enum HandleEventResult {
    KeepSession,
    RemoveSession,
}

impl DragSession {
    fn handle_event<'a>(
        &self,
        session_id: DragSessionId,
        env: &JNIEnv<'a>,
        event: DragEvent,
    ) -> NativeExtensionsResult<HandleEventResult> {
        let action = event.get_action(env)?;
        if action == DragAction::DragLocation {
            if let Some(delegate) = self.platform_context_delegate.upgrade() {
                delegate.drag_session_did_move_to_location(
                    self.platform_context_id,
                    session_id,
                    Point {
                        x: event.get_x(env)? as f64,
                        y: event.get_y(env)? as f64,
                    },
                );
            }
        }
        if action == DragAction::DragEnded {
            if let Some(delegate) = self.platform_context_delegate.upgrade() {
                let result = event.get_result(env)?;
                let operation = match result {
                    true => DropOperation::Copy, // TODO(knopp): Move?
                    false => DropOperation::None,
                };
                delegate.drag_session_did_end_with_operation(
                    self.platform_context_id,
                    session_id,
                    operation,
                );
            }
            Ok(HandleEventResult::RemoveSession)
        } else {
            Ok(HandleEventResult::KeepSession)
        }
    }
}

impl Drop for PlatformDragContext {
    fn drop(&mut self) {
        CONTEXTS.with(|c| c.borrow_mut().remove(&self.id));
    }
}
