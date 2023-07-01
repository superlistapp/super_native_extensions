use jni::{
    objects::{JObject, JString},
    JNIEnv,
};

use crate::error::NativeExtensionsError;

pub type JniResult<T> = jni::errors::Result<T>;

impl From<jni::errors::Error> for NativeExtensionsError {
    fn from(error: jni::errors::Error) -> Self {
        NativeExtensionsError::OtherError(format!("JNI: {error}"))
    }
}

pub fn jstring_from_utf8<'a>(env: &JNIEnv<'a>, data: &[u8]) -> JniResult<JString<'a>> {
    let string = String::from_utf8_lossy(data);
    env.new_string(string)
}

pub fn uri_from_utf8<'a>(env: &mut JNIEnv<'a>, data: &[u8]) -> JniResult<JObject<'a>> {
    uri_from_string(env, &String::from_utf8_lossy(data))
}

pub fn uri_from_string<'a>(env: &mut JNIEnv<'a>, string: &str) -> JniResult<JObject<'a>> {
    let string = env.new_string(string)?;
    env.call_static_method(
        "android/net/Uri",
        "parse",
        "(Ljava/lang/String;)Landroid/net/Uri;",
        &[(&string).into()],
    )?
    .l()
}
