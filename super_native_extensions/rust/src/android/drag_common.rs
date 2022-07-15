use jni::{objects::JObject, JNIEnv};

pub enum DragAction {
    DragStarted = 0x01,
    DragLocation = 0x02,
    Drop = 0x03,
    DragEnded = 0x04,
    DragEntered = 0x05,
    DragExited = 0x06,
}

struct DragEvent<'a>(JObject<'a>);

impl<'a> DragEvent<'a> {
    fn get_action(&self, env: &JNIEnv<'a>) -> DragAction {
        // env.call_method_unchecked(obj, method_id, ret, args)
        DragAction::DragEnded
    }
}