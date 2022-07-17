use jni::{objects::JObject, JNIEnv};

use crate::{
    android::DRAG_DROP_UTIL,
    drag_manager::DragSessionId,
    error::{NativeExtensionsError, NativeExtensionsResult},
};

#[derive(Debug, PartialEq)]
pub enum DragAction {
    DragStarted,
    DragLocation,
    Drop,
    DragEnded,
    DragEntered,
    DragExited,
}

#[derive(Clone, Copy, Debug)]
pub struct DragEvent<'a>(pub JObject<'a>);

impl<'a> DragEvent<'a> {
    pub fn get_action(&self, env: &JNIEnv<'a>) -> NativeExtensionsResult<DragAction> {
        let action = env.call_method(self.0, "getAction", "()I", &[])?.i()?;
        match action {
            0x01 => Ok(DragAction::DragStarted),
            0x02 => Ok(DragAction::DragLocation),
            0x03 => Ok(DragAction::Drop),
            0x04 => Ok(DragAction::DragEnded),
            0x05 => Ok(DragAction::DragEntered),
            0x06 => Ok(DragAction::DragExited),
            _ => Err(NativeExtensionsError::OtherError(format!(
                "Unknown drag action: {}",
                action
            ))),
        }
    }

    pub fn get_result(&self, env: &JNIEnv<'a>) -> NativeExtensionsResult<bool> {
        Ok(env.call_method(self.0, "getResult", "()Z", &[])?.z()?)
    }

    pub fn get_x(&self, env: &JNIEnv<'a>) -> NativeExtensionsResult<f32> {
        Ok(env.call_method(self.0, "getX", "()F", &[])?.f()?)
    }

    pub fn get_y(&self, env: &JNIEnv<'a>) -> NativeExtensionsResult<f32> {
        Ok(env.call_method(self.0, "getY", "()F", &[])?.f()?)
    }

    pub fn get_clip_description(&self, env: &JNIEnv<'a>) -> NativeExtensionsResult<JObject<'a>> {
        Ok(env
            .call_method(
                self.0,
                "getClipDescription",
                "()Landroid/content/ClipDescription;",
                &[],
            )?
            .l()?)
    }

    pub fn get_clip_data(&self, env: &JNIEnv<'a>) -> NativeExtensionsResult<JObject<'a>> {
        Ok(env
            .call_method(self.0, "getClipData", "()Landroid/content/ClipData;", &[])?
            .l()?)
    }

    pub fn get_session_id(
        &self,
        env: &JNIEnv<'a>,
    ) -> NativeExtensionsResult<Option<DragSessionId>> {
        let res = env
            .call_method(
                DRAG_DROP_UTIL.get().unwrap().as_obj(),
                "getSessionId",
                "(Landroid/view/DragEvent;)Ljava/lang/Long;",
                &[self.0.into()],
            )?
            .l()?;
        if res.is_null() {
            Ok(None)
        } else {
            let session_id = env.call_method(res, "longValue", "()J", &[])?.j()?;
            Ok(Some(session_id.into()))
        }
    }
}
