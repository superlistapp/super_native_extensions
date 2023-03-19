use std::{cell::RefCell, collections::HashMap, rc::Weak, sync::Arc};

use irondash_engine_context::EngineContext;
use irondash_message_channel::Value;
use jni::{objects::JObject, sys::jsize, JNIEnv};

use crate::{
    android::{DRAG_DROP_HELPER, JAVA_VM},
    api_model::{DataProviderId, DragConfiguration, DragRequest, DropOperation, ImageData, Point},
    data_provider_manager::DataProviderHandle,
    drag_manager::{
        DataProviderEntry, DragSessionId, PlatformDragContextDelegate, PlatformDragContextId,
    },
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    shadow::WithShadow,
};

use super::{
    drag_common::{DragAction, DragEvent},
    PlatformDataProvider,
};

pub struct PlatformDragContext {
    id: PlatformDragContextId,
    engine_handle: i64,
    delegate: Weak<dyn PlatformDragContextDelegate>,
    sessions: RefCell<HashMap<DragSessionId, DragSession>>,
}

struct DragSession {
    platform_context_id: PlatformDragContextId,
    configuration: DragConfiguration,
    platform_context_delegate: Weak<dyn PlatformDragContextDelegate>,
    data_providers: Vec<Arc<DataProviderHandle>>,
}

thread_local! {
    static CONTEXTS: RefCell<HashMap<PlatformDragContextId, Weak<PlatformDragContext>>> = RefCell::new(HashMap::new());
}

impl PlatformDragContext {
    pub fn new(
        id: PlatformDragContextId,
        engine_handle: i64,
        delegate: Weak<dyn PlatformDragContextDelegate>,
    ) -> NativeExtensionsResult<Self> {
        Ok(Self {
            id,
            engine_handle,
            delegate,
            sessions: RefCell::new(HashMap::new()),
        })
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

    pub fn needs_combined_drag_image() -> bool {
        true
    }

    pub async fn start_drag(
        &self,
        request: DragRequest,
        providers: HashMap<DataProviderId, DataProviderEntry>,
        session_id: DragSessionId,
    ) -> NativeExtensionsResult<()> {
        let env = JAVA_VM
            .get()
            .ok_or_else(|| NativeExtensionsError::OtherError("JAVA_VM not set".into()))?
            .attach_current_thread()?;

        let provider_handles: Vec<_> = providers.iter().map(|p| p.1.handle.clone()).collect();

        let providers: Vec<_> = request
            .configuration
            .items
            .iter()
            .map(|item| providers[&item.data_provider_id].provider.clone())
            .collect();

        let data = PlatformDataProvider::create_clip_data_for_data_providers(&env, providers)?;

        let image = &request.combined_drag_image.ok_or_else(|| {
            NativeExtensionsError::OtherError("Missing combined drag image".into())
        })?;
        let image = image.with_shadow(10);
        let bitmap = Self::create_bitmap(&env, &image.image_data)?;
        let device_pixel_ratio = image.image_data.device_pixel_ratio.unwrap_or(1.0);
        let point_in_rect = Point {
            x: (request.position.x - image.source_rect.x) * device_pixel_ratio,
            y: (request.position.y - image.source_rect.y) * device_pixel_ratio,
        };

        let mut sessions = self.sessions.borrow_mut();
        sessions.insert(
            session_id,
            DragSession {
                configuration: request.configuration,
                platform_context_id: self.id,
                platform_context_delegate: self.delegate.clone(),
                data_providers: provider_handles,
            },
        );

        let view = EngineContext::get()?.get_flutter_view(self.engine_handle)?;

        let session_id: i64 = session_id.into();
        env.call_method(
            DRAG_DROP_HELPER.get().unwrap().as_obj(),
            "startDrag",
            "(Lio/flutter/embedding/android/FlutterView;JLandroid/content/ClipData;Landroid/graphics/Bitmap;II)V",
            &[
                view.as_obj().into(),
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

    pub fn get_local_data<'a>(&self, env: &JNIEnv<'a>, event: DragEvent<'a>) -> Option<Vec<Value>> {
        let sessions = self.sessions.borrow();
        let session_id = event.get_session_id(env).ok_log()?;
        let session = session_id.and_then(|id| sessions.get(&id));

        session.map(|s| s.configuration.get_local_data())
    }

    pub fn get_local_data_for_session_id(
        &self,
        session_id: DragSessionId,
    ) -> NativeExtensionsResult<Vec<Value>> {
        let sessions = self.sessions.borrow();
        let session = sessions
            .get(&session_id)
            .ok_or(NativeExtensionsError::DragSessionNotFound)?;
        Ok(session.configuration.get_local_data())
    }

    pub fn get_data_provider_handles<'a>(
        &self,
        env: &JNIEnv<'a>,
        event: DragEvent<'a>,
    ) -> Option<Vec<Arc<DataProviderHandle>>> {
        let session_id = event.get_session_id(env).ok_log()?;
        match session_id {
            Some(session_id) => {
                let sessions = self.sessions.borrow();
                let session = sessions.get(&session_id);
                session.map(|s| s.data_providers.clone())
            }
            None => None,
        }
    }
}

#[derive(PartialEq)]
enum HandleEventResult {
    KeepSession,
    RemoveSession,
}

impl DragSession {
    fn handle_event(
        &self,
        session_id: DragSessionId,
        env: &JNIEnv,
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
