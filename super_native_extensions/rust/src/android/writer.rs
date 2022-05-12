use std::{
    cell::Cell,
    collections::HashMap,
    rc::Weak,
    sync::{Arc, Mutex},
};

use jni::{
    objects::{JClass, JObject, JString},
    sys::{jobject, jsize},
    JNIEnv,
};
use nativeshell_core::{
    util::{Capsule, Late},
    Context, IsolateId, RunLoopSender, Value,
};
use once_cell::sync::Lazy;
use url::Url;

use crate::{
    android::{CONTEXT, JAVA_VM},
    error::{ClipboardError, ClipboardResult},
    log::OkLog,
    value_coerce::{CoerceToData, StringFormat},
    value_promise::{ValuePromise, ValuePromiseResult},
    writer_data::{ClipboardWriterData, ClipboardWriterItem, ClipboardWriterItemData},
    writer_manager::PlatformClipboardWriterDelegate,
};

type JniResult<T> = jni::errors::Result<T>;

struct WriterRecord {
    data: ClipboardWriterData,
    delegate: Capsule<Weak<dyn PlatformClipboardWriterDelegate>>,
    isolate_id: IsolateId,
    sender: RunLoopSender,
}

static WRITERS: Lazy<Mutex<HashMap<i64, WriterRecord>>> = Lazy::new(|| Mutex::new(HashMap::new()));

thread_local! {
    static NEXT_ID: Cell<i64> = Cell::new(1);
}

pub struct PlatformClipboardWriter {
    weak_self: Late<Weak<Self>>,
    writer_id: i64,
}

// Compare mime type against another type or pattern; Use existing implementation for compatibility
fn compare_mime_types(env: &JNIEnv, concrete_type: &str, desired_type: &str) -> JniResult<bool> {
    if concrete_type == desired_type {
        return Ok(true);
    }
    env.call_static_method(
        "android/content/ClipDescription",
        "compareMimeTypes",
        "(Ljava/lang/String;Ljava/lang/String;)Z",
        &[
            env.new_string(concrete_type)?.into(),
            env.new_string(desired_type)?.into(),
        ],
    )?
    .z()
}

const MIME_TYPE_TEXT_PLAIN: &str = "text/plain";
const MIME_TYPE_TEXT_HTML: &str = "text/html";
const MIME_TYPE_URI_LIST: &str = "text/uri-list";

impl From<jni::errors::Error> for ClipboardError {
    fn from(error: jni::errors::Error) -> Self {
        ClipboardError::OtherError(format!("JNI: {}", error))
    }
}

fn contains(l: &[String], s: &str) -> bool {
    l.iter().any(|v| v == s)
}

impl PlatformClipboardWriter {
    pub fn new(
        delegate: Weak<dyn PlatformClipboardWriterDelegate>,
        isolate_id: IsolateId,
        data: ClipboardWriterData,
    ) -> Self {
        let id = NEXT_ID.with(|f| f.replace(f.get() + 1));
        let mut writers = WRITERS.lock().unwrap();
        let sender = Context::get().run_loop().new_sender();
        writers.insert(
            id,
            WriterRecord {
                data,
                delegate: Capsule::new_with_sender(delegate, sender.clone()),
                isolate_id,
                sender,
            },
        );
        Self {
            writer_id: id,
            weak_self: Late::new(),
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    fn jstring_from_utf8<'a>(env: &JNIEnv<'a>, data: &[u8]) -> JniResult<JString<'a>> {
        let string = String::from_utf8_lossy(data);
        env.new_string(string)
    }

    fn uri_from_utf8<'a>(env: &JNIEnv<'a>, data: &[u8]) -> JniResult<JObject<'a>> {
        Self::uri_from_string(env, &String::from_utf8_lossy(data))
    }

    fn uri_from_string<'a>(env: &JNIEnv<'a>, string: &str) -> JniResult<JObject<'a>> {
        let string = env.new_string(string)?;
        env.call_static_method(
            "android/net/Uri",
            "parse",
            "(Ljava/lang/String;)Landroid/net/Uri;",
            &[string.into()],
        )?
        .l()
    }

    fn content_provider_uri<'a>(
        env: &JNIEnv<'a>,
        writer_id: i64,
        index: usize,
    ) -> ClipboardResult<JObject<'a>> {
        let context = CONTEXT
            .get()
            .ok_or_else(|| ClipboardError::OtherError("Missing Android Context".into()))?
            .as_obj();
        let package_name = env
            .call_method(context, "getPackageName", "()Ljava/lang/String;", &[])?
            .l()?;
        let package_name: String = env.get_string(package_name.into())?.into();
        let uri = format!(
            "content://{}.ClipboardDataProvider/{}/{}",
            package_name, writer_id, index
        );
        Ok(Self::uri_from_string(env, &uri)?)
    }

    fn create_clip_item_for_writer_item<'a>(
        env: &JNIEnv<'a>,
        writer_id: i64,
        item: &ClipboardWriterItem,
        index: usize,
        clipboard_mime_types: &mut Vec<String>,
    ) -> ClipboardResult<Option<JObject<'a>>> {
        let mut text = None::<JObject>;
        let mut text_html = None::<JObject>;
        let mut uri = None::<JObject>;

        for data in &item.data {
            match data {
                ClipboardWriterItemData::Simple { types, data } => {
                    let data = data.coerce_to_data(StringFormat::Utf8).unwrap_or_default();
                    for ty in types {
                        match ty.as_str() {
                            MIME_TYPE_TEXT_PLAIN => {
                                text = Some(Self::jstring_from_utf8(env, &data)?.into());
                                if !contains(clipboard_mime_types, MIME_TYPE_TEXT_PLAIN) {
                                    clipboard_mime_types.push(MIME_TYPE_TEXT_PLAIN.into());
                                }
                            }
                            MIME_TYPE_TEXT_HTML => {
                                text_html = Some(Self::jstring_from_utf8(env, &data)?.into());
                                if !contains(clipboard_mime_types, MIME_TYPE_TEXT_HTML) {
                                    clipboard_mime_types.push(MIME_TYPE_TEXT_HTML.into());
                                }
                            }
                            MIME_TYPE_URI_LIST => {
                                if uri.is_none() {
                                    // do not replace UI, might be a content URI
                                    uri = Some(Self::uri_from_utf8(env, &data)?);
                                }
                                if !contains(clipboard_mime_types, MIME_TYPE_URI_LIST) {
                                    clipboard_mime_types.push(MIME_TYPE_URI_LIST.into());
                                }
                            }
                            other_type => {
                                uri = Some(Self::content_provider_uri(env, writer_id, index)?);
                                if !contains(clipboard_mime_types, other_type) {
                                    clipboard_mime_types.push(other_type.into())
                                }
                            }
                        }
                    }
                }
                ClipboardWriterItemData::Lazy { types, id: _ } => {
                    for ty in types {
                        if !contains(clipboard_mime_types, ty) {
                            clipboard_mime_types.push(ty.into())
                        }
                        // always use URI for lazy data
                        uri = Some(Self::content_provider_uri(env, writer_id, index)?);
                    }
                }
                _ => {}
            }
        }

        if text.is_none() && text_html.is_some() {
            return Err(ClipboardError::OtherError(
                "You must provide plain text fallback for HTML clipboard text".into(),
            ));
        }

        if text.is_some() || text_html.is_some() || uri.is_some() {
            let obj = env.new_object(
                "android/content/ClipData$Item",
                "(Ljava/lang/CharSequence;Ljava/lang/String;Landroid/content/Intent;Landroid/net/Uri;)V",
                &[
                    text.unwrap_or_else(JObject::null).into(),
                    text_html.unwrap_or_else(JObject::null).into(),
                    JObject::null().into(),
                    uri.unwrap_or_else(JObject::null).into()
                ])?;
            Ok(Some(obj))
        } else {
            Ok(None)
        }
    }

    fn create_clip_data_for_writer<'a>(
        env: &JNIEnv<'a>,
        writer_id: i64,
        writer: &ClipboardWriterData,
    ) -> ClipboardResult<JObject<'a>> {
        let mut clipboard_mime_types = Vec::<String>::new();
        let mut items = Vec::<JObject>::new();
        for (index, item) in writer.items.iter().enumerate() {
            let item = Self::create_clip_item_for_writer_item(
                env,
                writer_id,
                item,
                index,
                &mut clipboard_mime_types,
            )?;
            if let Some(item) = item {
                items.push(item);
            }
        }

        let types = env.new_object_array(
            clipboard_mime_types.len() as i32,
            "java/lang/String",
            JObject::null(),
        )?;
        for (i, ty) in clipboard_mime_types.iter().enumerate() {
            env.set_object_array_element(types, i as i32, env.new_string(ty)?)?;
        }

        let clip_description = env.new_object(
            "android/content/ClipDescription",
            "(Ljava/lang/CharSequence;[Ljava/lang/String;)V",
            &[env.new_string("Clip")?.into(), types.into()],
        )?;

        let mut clip_data = JObject::null();

        for item in items {
            if clip_data.is_null() {
                clip_data = env.new_object(
                    "android/content/ClipData",
                    "(Landroid/content/ClipDescription;Landroid/content/ClipData$Item;)V",
                    &[clip_description.into(), item.into()],
                )?;
            } else {
                env.call_method(
                    clip_data,
                    "addItem",
                    "(Landroid/content/ClipData$Item;)V",
                    &[item.into()],
                )?;
            }
        }

        Ok(clip_data)
    }

    pub async fn write_to_clipboard(&self) -> ClipboardResult<()> {
        let writers = WRITERS.lock().unwrap();
        let writer = writers.get(&self.writer_id);
        if let Some(writer) = writer.map(|s| &s.data) {
            let env = JAVA_VM
                .get()
                .ok_or_else(|| ClipboardError::OtherError("JAVA_VM not set".into()))?
                .attach_current_thread()?;
            let context = CONTEXT.get().unwrap().as_obj();
            let clipboard_service = env
                .get_static_field(
                    env.find_class("android/content/Context")?,
                    "CLIPBOARD_SERVICE",
                    "Ljava/lang/String;",
                )?
                .l()?;
            let clipboard_manager = env
                .call_method(
                    context,
                    "getSystemService",
                    "(Ljava/lang/String;)Ljava/lang/Object;",
                    &[clipboard_service.into()],
                )?
                .l()?;
            let clip_data = Self::create_clip_data_for_writer(&env, self.writer_id, writer)?;
            env.call_method(
                clipboard_manager,
                "setPrimaryClip",
                "(Landroid/content/ClipData;)V",
                &[clip_data.into()],
            )?;
        }
        Ok(())
    }
}

impl Drop for PlatformClipboardWriter {
    fn drop(&mut self) {
        let mut writers = WRITERS.lock().unwrap();
        writers.remove(&self.writer_id);
    }
}

#[derive(Debug)]
struct UriInfo {
    writer_id: i64,
    index: usize,
}

impl UriInfo {
    fn parse(env: &JNIEnv, uri_string: JString) -> Option<UriInfo> {
        let uri = env.get_string(uri_string).ok()?;
        let uri = Url::parse(&uri.to_string_lossy()).ok()?;
        let mut path_segments = uri.path_segments()?;

        let writer_id = path_segments.next()?;
        let writer_id = writer_id.parse::<i64>().ok()?;

        let index = path_segments.next()?;
        let index = index.parse::<usize>().ok()?;

        Some(UriInfo { writer_id, index })
    }
}

fn get_mime_types_for_uri<'a>(
    env: &JNIEnv<'a>,
    uri_string: JString,
    filter: JString,
) -> ClipboardResult<JObject<'a>> {
    let info = UriInfo::parse(env, uri_string)
        .ok_or_else(|| ClipboardError::OtherError("Malformed URI".into()))?;

    let filter = env.get_string(filter)?;
    let filter = filter.to_string_lossy();

    let mut mime_types = Vec::<String>::new();

    let writers = WRITERS.lock().unwrap();
    let writer = writers.get(&info.writer_id);
    if let Some(writer) = writer {
        let item = &writer.data.items.get(info.index);
        if let Some(item) = item {
            for data in item.data.iter() {
                match data {
                    ClipboardWriterItemData::Simple { types, data: _ } => {
                        for ty in types {
                            if compare_mime_types(env, ty, &filter)? {
                                mime_types.push(ty.to_owned())
                            }
                        }
                    }
                    ClipboardWriterItemData::Lazy { types, id: _ } => {
                        for ty in types {
                            if compare_mime_types(env, ty, &filter)? {
                                mime_types.push(ty.to_owned())
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let res: JObject = env
        .new_object_array(
            mime_types.len() as jsize,
            "java/lang/String",
            JObject::null(),
        )
        .expect("Failed to create String[]")
        .into();

    for (i, str) in mime_types.iter().enumerate() {
        let string = env.new_string(str).expect("Failed to create String");
        env.set_object_array_element(*res, i as i32, string)
            .unwrap();
    }
    Ok(res)
}

fn get_value(env: &JNIEnv, promise: Arc<ValuePromise>) -> ClipboardResult<ValuePromiseResult> {
    if Context::current().is_some() {
        // this is main thread - we need to poll the event loop while waiting
        let context = CONTEXT.get().unwrap().as_obj();
        let looper = env
            .call_method(context, "getMainLooper", "()Landroid/os/Looper;", &[])?
            .l()?;
        let queue = env
            .call_method(looper, "getQueue", "()Landroid/os/MessageQueue;", &[])?
            .l()?;
        loop {
            if let Some(result) = promise.try_take() {
                return Ok(result);
            }
            let message = env
                .call_method(queue, "next", "()Landroid/os/Message;", &[])?
                .l()?;

            if message.is_null() {
                return Ok(ValuePromiseResult::Cancelled);
            } else {
                let target = env
                    .call_method(message, "getTarget", "()Landroid/os/Handler;", &[])?
                    .l()?;
                if target.is_null() {
                    return Ok(ValuePromiseResult::Cancelled);
                } else {
                    env.call_method(
                        target,
                        "dispatchMessage",
                        "(Landroid/os/Message;)V",
                        &[message.into()],
                    )?;
                }
            }
        }
    } else {
        Ok(promise.wait())
    }
}

fn get_data_for_uri<'a>(
    env: &JNIEnv<'a>,
    this: JClass,
    uri_string: JString,
    mime_type: JString,
) -> ClipboardResult<JObject<'a>> {
    fn byte_array_from_value<'a>(env: &JNIEnv<'a>, value: &Value) -> ClipboardResult<JObject<'a>> {
        let data = value.coerce_to_data(StringFormat::Utf8).unwrap_or_default();
        let res: JObject = env.new_byte_array(data.len() as i32).unwrap().into();
        let data: &[u8] = &data;
        env.set_byte_array_region(*res, 0, unsafe { std::mem::transmute(data) })?;
        Ok(res)
    }

    let info = UriInfo::parse(env, uri_string)
        .ok_or_else(|| ClipboardError::OtherError("Malformed URI".into()))?;

    let mime_type = env.get_string(mime_type)?;
    let mime_type = mime_type.to_string_lossy();

    let writers = WRITERS.lock().unwrap();
    let writer = writers.get(&info.writer_id);
    if let Some(writer) = writer {
        let item = &writer.data.items.get(info.index);
        if let Some(item) = item {
            for data in item.data.iter() {
                match data {
                    ClipboardWriterItemData::Simple { types, data } => {
                        if contains(types, &mime_type) {
                            return byte_array_from_value(env, data);
                        }
                    }
                    ClipboardWriterItemData::Lazy { types, id } => {
                        if contains(types, &mime_type) {
                            let delegate = writer.delegate.clone();
                            let isolate_id = writer.isolate_id;
                            let id = *id;
                            let class = env.new_global_ref(this)?;
                            let value = writer.sender.send_and_wait(move || {
                                delegate.get_ref().unwrap().upgrade().map(|delegate| {
                                    delegate.get_lazy_data(
                                        isolate_id,
                                        id,
                                        // Wake up the android part of the looper so that polling
                                        // above will continue (normally RunLoopSender only wakes up the
                                        // native part of Looper).
                                        Some(Box::new(move || {
                                            let env = JAVA_VM
                                                .get()
                                                .unwrap()
                                                .attach_current_thread()
                                                .unwrap();
                                            env.call_method(class.as_obj(), "wakeUp", "()V", &[])
                                                .ok_log();
                                        })),
                                    )
                                })
                            });
                            match value {
                                Some(value) => {
                                    let res = get_value(env, value)?;
                                    match res {
                                        ValuePromiseResult::Ok { value } => {
                                            return byte_array_from_value(env, &value);
                                        }
                                        ValuePromiseResult::Cancelled => return Ok(JObject::null()),
                                    }
                                }
                                None => return Ok(JObject::null()),
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(JObject::null())
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn Java_com_superlist_super_1data_1transfer_DataProvider_getAllMimeTypesForURI(
    env: JNIEnv,
    _this: JClass,
    uri_string: JString,
    filter: JString,
) -> jobject {
    let res = get_mime_types_for_uri(&env, uri_string, filter);
    match res {
        Ok(res) => res.into_inner(),
        Err(err) => {
            log::error!("{}", err);
            JObject::null().into_inner()
        }
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn Java_com_superlist_super_1data_1transfer_DataProvider_getDataForURI(
    env: JNIEnv,
    this: JClass,
    uri_string: JString,
    mime_type: JString,
) -> jobject {
    let res = get_data_for_uri(&env, this, uri_string, mime_type);
    match res {
        Ok(res) => res.into_inner(),
        Err(err) => {
            log::error!("{}", err);
            JObject::null().into_inner()
        }
    }
}
