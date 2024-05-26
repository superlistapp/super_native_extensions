use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
};

use irondash_message_channel::{IsolateId, Late, Value};
use irondash_run_loop::{util::Capsule, RunLoop, RunLoopSender};
use jni::{
    objects::{JClass, JObject, JString},
    sys::{jobject, jsize},
    JNIEnv,
};

use once_cell::sync::Lazy;
use url::Url;

use crate::{
    android::{CONTEXT, JAVA_VM},
    api_model::{DataProvider, DataRepresentation},
    context::Context,
    data_provider_manager::{DataProviderHandle, PlatformDataProviderDelegate},
    error::{NativeExtensionsError, NativeExtensionsResult},
    util::NextId,
    value_coerce::{CoerceToData, StringFormat},
    value_promise::{ValuePromise, ValuePromiseResult},
};

use super::util::{jstring_from_utf8, uri_from_string, uri_from_utf8};

type JniResult<T> = jni::errors::Result<T>;

struct DataProviderRecord {
    data: DataProvider,
    delegate: Capsule<Weak<dyn PlatformDataProviderDelegate>>,
    isolate_id: IsolateId,
    sender: RunLoopSender,
}

static DATA_PROVIDERS: Lazy<Mutex<HashMap<i64, DataProviderRecord>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

thread_local! {
    static NEXT_ID: Cell<i64> = const { Cell::new(1) };
}

pub struct PlatformDataProvider {
    weak_self: Late<Weak<Self>>,
    data_provider_id: i64,
}

// Compare mime type against another type or pattern; Use existing implementation for compatibility
fn compare_mime_types(
    env: &mut JNIEnv,
    concrete_type: &str,
    desired_type: &str,
) -> JniResult<bool> {
    if concrete_type == desired_type {
        return Ok(true);
    }
    env.call_static_method(
        "android/content/ClipDescription",
        "compareMimeTypes",
        "(Ljava/lang/String;Ljava/lang/String;)Z",
        &[
            (&env.new_string(concrete_type)?).into(),
            (&env.new_string(desired_type)?).into(),
        ],
    )?
    .z()
}

pub fn platform_stream_write(_handle: i32, _data: &[u8]) -> i32 {
    1
}

pub fn platform_stream_close(_handle: i32, _delete: bool) {}

pub const MIME_TYPE_TEXT_PLAIN: &str = "text/plain";
pub const MIME_TYPE_TEXT_HTML: &str = "text/html";
pub const MIME_TYPE_URI_LIST: &str = "text/uri-list";

fn contains(l: &[String], s: &str) -> bool {
    l.iter().any(|v| v == s)
}

impl PlatformDataProvider {
    pub fn new(
        delegate: Weak<dyn PlatformDataProviderDelegate>,
        isolate_id: IsolateId,
        data: DataProvider,
    ) -> Self {
        let id = NEXT_ID.with(|f| f.next_id());
        let mut data_providers = DATA_PROVIDERS.lock().unwrap();
        let sender = RunLoop::current().new_sender();
        data_providers.insert(
            id,
            DataProviderRecord {
                data,
                delegate: Capsule::new_with_sender(delegate, sender.clone()),
                isolate_id,
                sender,
            },
        );
        Self {
            data_provider_id: id,
            weak_self: Late::new(),
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    fn content_provider_uri<'a>(
        env: &mut JNIEnv<'a>,
        data_source_id: i64,
    ) -> NativeExtensionsResult<JObject<'a>> {
        let context = CONTEXT
            .get()
            .ok_or_else(|| NativeExtensionsError::OtherError("Missing Android Context".into()))?
            .as_obj();
        let package_name = env
            .call_method(context, "getPackageName", "()Ljava/lang/String;", &[])?
            .l()?;
        let package_name: String = env.get_string((&package_name).into())?.into();
        let uri = format!("content://{package_name}.SuperClipboardDataProvider/{data_source_id}",);
        Ok(uri_from_string(env, &uri)?)
    }

    fn create_clip_item_for_data_provider<'a>(
        env: &mut JNIEnv<'a>,
        data_provider_id: i64,
        data_provider: &DataProvider,
        clipboard_mime_types: &mut Vec<String>,
    ) -> NativeExtensionsResult<Option<JObject<'a>>> {
        let mut text = None::<JObject<'a>>;
        let mut text_html = None::<JObject<'a>>;
        let mut uri = None::<JObject<'a>>;

        for repr in &data_provider.representations {
            match repr {
                DataRepresentation::Simple { format, data } => {
                    let data = data.coerce_to_data(StringFormat::Utf8).unwrap_or_default();
                    match format.as_str() {
                        MIME_TYPE_TEXT_PLAIN => {
                            text = Some(jstring_from_utf8(env, &data)?.into());
                            if !contains(clipboard_mime_types, MIME_TYPE_TEXT_PLAIN) {
                                clipboard_mime_types.push(MIME_TYPE_TEXT_PLAIN.into());
                            }
                        }
                        MIME_TYPE_TEXT_HTML => {
                            text_html = Some(jstring_from_utf8(env, &data)?.into());
                            if !contains(clipboard_mime_types, MIME_TYPE_TEXT_HTML) {
                                clipboard_mime_types.push(MIME_TYPE_TEXT_HTML.into());
                            }
                        }
                        MIME_TYPE_URI_LIST => {
                            if uri.is_none() {
                                // do not replace URI, might be a content URI
                                uri = Some(uri_from_utf8(env, &data)?);
                            }
                            if !contains(clipboard_mime_types, MIME_TYPE_URI_LIST) {
                                clipboard_mime_types.push(MIME_TYPE_URI_LIST.into());
                            }
                        }
                        other_type => {
                            uri = Some(Self::content_provider_uri(env, data_provider_id)?);
                            if !contains(clipboard_mime_types, other_type) {
                                clipboard_mime_types.push(other_type.into())
                            }
                        }
                    }
                }
                DataRepresentation::Lazy { format, id: _ } => {
                    if !contains(clipboard_mime_types, format) {
                        clipboard_mime_types.push(format.into())
                    }
                    // always use URI for lazy data
                    uri = Some(Self::content_provider_uri(env, data_provider_id)?);
                }
                _ => {}
            }
        }

        if text.is_none() && text_html.is_some() {
            return Err(NativeExtensionsError::OtherError(
                "You must provide plain text fallback for HTML clipboard text".into(),
            ));
        }

        if text.is_some() || text_html.is_some() || uri.is_some() {
            let obj = env.new_object(
                "android/content/ClipData$Item",
                "(Ljava/lang/CharSequence;Ljava/lang/String;Landroid/content/Intent;Landroid/net/Uri;)V",
                &[
                    (&text.unwrap_or_default()).into(),
                    (&text_html.unwrap_or_default()).into(),
                    (&JObject::null()).into(),
                    (&uri.unwrap_or_default()).into()
                ])?;
            Ok(Some(obj))
        } else {
            Ok(None)
        }
    }

    pub fn create_clip_data_for_data_providers<'a>(
        env: &mut JNIEnv<'a>,
        providers: Vec<Rc<PlatformDataProvider>>,
    ) -> NativeExtensionsResult<JObject<'a>> {
        let data_providers = DATA_PROVIDERS.lock().unwrap();
        let providers: Vec<_> = providers
            .iter()
            .map(|provider| {
                (
                    provider.data_provider_id,
                    &data_providers[&provider.data_provider_id].data,
                )
            })
            .collect();
        Self::_create_clip_data_for_data_providers(env, providers)
    }

    fn _create_clip_data_for_data_providers<'a>(
        env: &mut JNIEnv<'a>,
        providers: Vec<(i64, &DataProvider)>,
    ) -> NativeExtensionsResult<JObject<'a>> {
        let mut clipboard_mime_types = Vec::<String>::new();
        let mut items = Vec::<JObject>::new();
        for (provider_id, provider) in providers.iter() {
            let item = Self::create_clip_item_for_data_provider(
                env,
                *provider_id,
                provider,
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
            env.set_object_array_element(&types, i as i32, env.new_string(ty)?)?;
        }

        let clip_description = env.new_object(
            "android/content/ClipDescription",
            "(Ljava/lang/CharSequence;[Ljava/lang/String;)V",
            &[(&env.new_string("Clip")?).into(), (&types).into()],
        )?;

        let mut clip_data = JObject::null();

        for item in items {
            if env.is_same_object(&clip_data, JObject::null())? {
                clip_data = env.new_object(
                    "android/content/ClipData",
                    "(Landroid/content/ClipDescription;Landroid/content/ClipData$Item;)V",
                    &[(&clip_description).into(), (&item).into()],
                )?;
            } else {
                env.call_method(
                    &clip_data,
                    "addItem",
                    "(Landroid/content/ClipData$Item;)V",
                    &[(&item).into()],
                )?;
            }
        }

        Ok(clip_data)
    }

    pub async fn write_to_clipboard(
        providers: Vec<(Rc<PlatformDataProvider>, Arc<DataProviderHandle>)>,
    ) -> NativeExtensionsResult<()> {
        let handles: Vec<_> = providers.iter().map(|p| p.1.clone()).collect();
        let providers: Vec<_> = providers.into_iter().map(|p| p.0).collect();

        thread_local! {
            static CURRENT_CLIP: RefCell<Vec<Arc<DataProviderHandle>>> = const { RefCell::new(Vec::new()) };
        }
        // ClipManager doesn't provide any lifetime management for clip so just
        // keep the data awake until the clip is replaced.
        CURRENT_CLIP.with(|r| r.replace(handles));

        let mut env = JAVA_VM
            .get()
            .ok_or_else(|| NativeExtensionsError::OtherError("JAVA_VM not set".into()))?
            .attach_current_thread()?;

        let clip_data = Self::create_clip_data_for_data_providers(&mut env, providers)?;

        let context = CONTEXT.get().unwrap().as_obj();
        let context_class = env.find_class("android/content/Context")?;
        let clipboard_service = env
            .get_static_field(context_class, "CLIPBOARD_SERVICE", "Ljava/lang/String;")?
            .l()?;
        let clipboard_manager = env
            .call_method(
                context,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[(&clipboard_service).into()],
            )?
            .l()?;
        env.call_method(
            clipboard_manager,
            "setPrimaryClip",
            "(Landroid/content/ClipData;)V",
            &[(&clip_data).into()],
        )?;

        Ok(())
    }
}

impl Drop for PlatformDataProvider {
    fn drop(&mut self) {
        let mut data_providers = DATA_PROVIDERS.lock().unwrap();
        data_providers.remove(&self.data_provider_id);
    }
}

#[derive(Debug)]
struct UriInfo {
    data_provider_id: i64,
}

impl UriInfo {
    fn parse(env: &mut JNIEnv, uri_string: JString) -> Option<UriInfo> {
        let uri = env.get_string(&uri_string).ok()?;
        let uri = Url::parse(&uri.to_string_lossy()).ok()?;
        let mut path_segments = uri.path_segments()?;

        let data_provider_id = path_segments.next()?;
        let data_source_id = data_provider_id.parse::<i64>().ok()?;

        Some(UriInfo {
            data_provider_id: data_source_id,
        })
    }
}

fn get_mime_types_for_uri<'a>(
    env: &mut JNIEnv<'a>,
    uri_string: JString,
    filter: JString,
) -> NativeExtensionsResult<JObject<'a>> {
    let info = UriInfo::parse(env, uri_string)
        .ok_or_else(|| NativeExtensionsError::OtherError("Malformed URI".into()))?;

    let filter = env.get_string(&filter)?;
    let filter = filter.to_string_lossy();

    let mut mime_types = Vec::<String>::new();

    let data_providers = DATA_PROVIDERS.lock().unwrap();
    let data_provider = data_providers.get(&info.data_provider_id);
    if let Some(data_provider) = data_provider {
        for repr in &data_provider.data.representations {
            match repr {
                DataRepresentation::Simple { format, data: _ } => {
                    if compare_mime_types(env, format, &filter)? {
                        mime_types.push(format.to_owned())
                    }
                }
                DataRepresentation::Lazy { format, id: _ } => {
                    if compare_mime_types(env, format, &filter)? {
                        mime_types.push(format.to_owned())
                    }
                }
                _ => {}
            }
        }
    }

    let res = env
        .new_object_array(
            mime_types.len() as jsize,
            "java/lang/String",
            JObject::null(),
        )
        .expect("Failed to create String[]");

    for (i, str) in mime_types.iter().enumerate() {
        let string = env.new_string(str).expect("Failed to create String");
        env.set_object_array_element(&res, i as i32, string)
            .unwrap();
    }
    Ok(res.into())
}

fn get_value(promise: Arc<ValuePromise>) -> NativeExtensionsResult<ValuePromiseResult> {
    if Context::current().is_some() {
        loop {
            if let Some(result) = promise.try_take() {
                return Ok(result);
            }
            RunLoop::current().platform_run_loop.poll_once();
        }
    } else {
        Ok(promise.wait())
    }
}

fn get_data_for_uri<'a>(
    env: &mut JNIEnv<'a>,
    _this: JClass,
    uri_string: JString,
    mime_type: JString,
) -> NativeExtensionsResult<JObject<'a>> {
    fn byte_array_from_value<'a>(
        env: &JNIEnv<'a>,
        value: &Value,
    ) -> NativeExtensionsResult<JObject<'a>> {
        let data = value.coerce_to_data(StringFormat::Utf8).unwrap_or_default();
        let res = env.new_byte_array(data.len() as i32).unwrap();
        let data: &[u8] = &data;
        env.set_byte_array_region(&res, 0, unsafe {
            std::mem::transmute::<&[u8], &[i8]>(data)
        })?;
        Ok(res.into())
    }

    let info = UriInfo::parse(env, uri_string)
        .ok_or_else(|| NativeExtensionsError::OtherError("Malformed URI".into()))?;

    let mime_type = env.get_string(&mime_type)?;
    let mime_type: String = mime_type.to_string_lossy().into();

    let data_providers = DATA_PROVIDERS.lock().unwrap();
    let data_provider = data_providers.get(&info.data_provider_id);
    if let Some(data_provider) = data_provider {
        for data in &data_provider.data.representations {
            match data {
                DataRepresentation::Simple { format, data } => {
                    if format == &mime_type {
                        return byte_array_from_value(env, data);
                    }
                }
                DataRepresentation::Lazy { format, id } => {
                    if format == &mime_type {
                        let delegate = data_provider.delegate.clone();
                        let isolate_id = data_provider.isolate_id;
                        let id = *id;
                        let value = data_provider.sender.send_and_wait(move || {
                            delegate
                                .get_ref()
                                .unwrap()
                                .upgrade()
                                .map(|delegate| delegate.get_lazy_data(isolate_id, id, None))
                        });
                        drop(data_providers);
                        match value {
                            Some(value) => {
                                let res = get_value(value)?;
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

    Ok(JObject::null())
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn Java_com_superlist_super_1native_1extensions_DataProvider_getAllMimeTypesForURI(
    mut env: JNIEnv,
    _this: JClass,
    uri_string: JString,
    filter: JString,
) -> jobject {
    let res = get_mime_types_for_uri(&mut env, uri_string, filter);
    match res {
        Ok(res) => res.as_raw(),
        Err(err) => {
            log::error!("{}", err);
            JObject::null().as_raw()
        }
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn Java_com_superlist_super_1native_1extensions_DataProvider_getDataForURI(
    mut env: JNIEnv,
    this: JClass,
    uri_string: JString,
    mime_type: JString,
) -> jobject {
    let res = get_data_for_uri(&mut env, this, uri_string, mime_type);
    match res {
        Ok(res) => res.as_raw(),
        Err(err) => {
            log::error!("{}", err);
            JObject::null().as_raw()
        }
    }
}
