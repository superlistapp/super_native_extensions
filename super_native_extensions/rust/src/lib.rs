#![allow(clippy::single_match)]
#![allow(clippy::comparison_chain)]

use std::ffi::c_void;

use ::log::debug;
use drag_drop_manager::GetDragDropManager;
use nativeshell_core::{nativeshell_init_message_channel_context, Context, FunctionResult};
use reader_manager::GetClipboardReaderManager;
use writer_manager::GetClipboardWriterManager;

mod api_model;
mod drag_drop_manager;
mod error;
mod log;
mod reader_manager;
mod value_coerce;
mod value_promise;
mod writer_data;
mod writer_manager;

#[cfg(not(test))]
#[path = "."]
mod platform_impl {
    #[cfg(target_os = "macos")]
    #[path = "macos/mod.rs"]
    pub mod platform;

    #[cfg(target_os = "ios")]
    #[path = "ios/mod.rs"]
    pub mod platform;

    #[cfg(target_os = "android")]
    #[path = "android/mod.rs"]
    pub mod platform;

    #[cfg(target_os = "windows")]
    #[path = "win32/mod.rs"]
    pub mod platform;

    #[cfg(target_os = "linux")]
    #[path = "linux/mod.rs"]
    pub mod platform;
}

#[cfg(test)]
#[path = "."]
mod platform_impl {
    #[path = "mock/mod.rs"]
    pub mod platform;
}

mod platform {
    pub(crate) use super::platform_impl::platform::*;
}

struct DataTransferPlugin {
    _context: Context,
}

impl DataTransferPlugin {
    fn new() -> Self {
        let context = Context::new();
        // eagerly initialize
        context.clipboard_writer_manager();
        context.clipboard_reader_manager();
        context.drag_drop_manager();
        DataTransferPlugin { _context: context }
    }
}

thread_local! {
    static PLUGIN: DataTransferPlugin = DataTransferPlugin::new();
}

fn init(init_loger: bool) {
    if init_loger {
        #[cfg(not(target_os = "ios"))]
        {
            simple_logger::init_with_level(::log::Level::Info).ok();
        }
        #[cfg(target_os = "ios")]
        {
            oslog::OsLogger::new("dev.nativeshell.clipboard")
                .level_filter(::log::LevelFilter::Debug)
                .init()
                .ok();
        }
    }
    // Lazily initialize the thread local
    PLUGIN.with(|_| {});
}

#[no_mangle]
pub extern "C" fn super_native_extensions_init() {
    init(true);
}

#[cfg(target_os = "android")]
mod android {
    use once_cell::sync::OnceCell;

    use crate::init;

    pub static JAVA_VM: OnceCell<jni::JavaVM> = OnceCell::new();
    pub static CONTEXT: OnceCell<jni::objects::GlobalRef> = OnceCell::new();

    // JNI class loader can't load our classes, so we store the data util instance;
    // If there were more classes to load we could store the class loader instead
    pub static CLIP_DATA_UTIL: OnceCell<jni::objects::GlobalRef> = OnceCell::new();
    pub static DRAG_DROP_UTIL: OnceCell<jni::objects::GlobalRef> = OnceCell::new();

    #[no_mangle]
    #[allow(non_snake_case)]
    pub extern "C" fn Java_com_superlist_super_1native_1extensions_SuperNativeExtensionsPlugin_init(
        env: jni::JNIEnv,
        _class: jni::objects::JClass,
        context: jni::objects::JObject,
        clip_data_util: jni::objects::JObject,
        drag_drop_util: jni::objects::JObject,
    ) {
        use ::log::Level;
        use android_logger::Config;

        android_logger::init_once(
            Config::default()
                .with_min_level(Level::Info)
                .with_tag("flutter"),
        );
        JAVA_VM.get_or_init(|| {
            env.get_java_vm()
                .expect("Failed to obtain JavaVM from JNIEnv")
        });
        CONTEXT.get_or_init(|| {
            env.new_global_ref(context)
                .expect("Failed to create Context reference")
        });
        CLIP_DATA_UTIL.get_or_init(|| {
            env.new_global_ref(clip_data_util)
                .expect("Failed to store clip data util")
        });
        DRAG_DROP_UTIL.get_or_init(|| {
            env.new_global_ref(drag_drop_util)
                .expect("Failed to store drag drop util")
        });
        init(false);
    }
}

#[no_mangle]
pub extern "C" fn super_native_extensions_init_message_channel_context(
    data: *mut c_void,
) -> FunctionResult {
    debug!("Initializing message channel context");
    nativeshell_init_message_channel_context(data)
}
