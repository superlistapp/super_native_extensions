#![allow(clippy::comparison_chain)]
#![allow(clippy::new_without_default)]
#![allow(clippy::single_match)]
#![allow(clippy::type_complexity)]
#![allow(unknown_lints)]
// https://github.com/rust-lang/rust-clippy/issues/11076
#![allow(clippy::arc_with_non_send_sync)]
// TODO(knopp): Fine grained way to prevent dead code warnings in code that is not used on all platforms.
#![allow(dead_code)]

use std::ffi::c_void;

use ::log::debug;
use clipboard_reader::GetClipboardReader;
use clipboard_writer::GetClipboardWriter;
use context::Context;
use data_provider_manager::GetDataProviderManager;
use drag_manager::GetDragManager;
use drop_manager::GetDropManager;
use hot_key_manager::GetHotKeyManager;
use keyboard_layout_manager::GetKeyboardLayoutDelegate;
use menu_manager::GetMenuManager;

use irondash_message_channel::{irondash_init_message_channel_context, FunctionResult};
use reader_manager::GetDataReaderManager;

mod api_model;
mod blur;
mod clipboard_reader;
mod clipboard_writer;
mod context;
mod data_provider_manager;
mod drag_manager;
mod drop_manager;
mod error;
mod hot_key_manager;
mod keyboard_layout_manager;
mod log;
mod menu_manager;
mod reader_manager;
mod shadow;
mod util;
mod value_coerce;
mod value_promise;

#[allow(dead_code)]
mod segmented_queue;

// #[cfg(not(test))]
#[path = "."]
mod platform_impl {
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    #[path = "darwin/mod.rs"]
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

// #[cfg(test)]
// #[path = "."]
// mod platform_impl {
//     #[path = "mock/mod.rs"]
//     pub mod platform;
// }

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
        context.data_provider_manager();
        context.data_reader_manager();
        context.clipboard_writer();
        context.clipboard_reader();
        context.drag_manager();
        context.drop_manager();
        context.keyboard_map_manager();
        context.hot_key_manager();
        context.menu_manager();
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
            oslog::OsLogger::new("supernativeextensions")
                .level_filter(::log::LevelFilter::Debug)
                .init()
                .ok();
        }
    }
    // Lazily initialize the thread local
    PLUGIN.with(|_| {});
}

#[no_mangle]
/// Entry point used for all non-android platforms
#[cfg(not(target_os = "android"))]
pub extern "C" fn super_native_extensions_init() {
    init(true);
}

#[cfg(target_os = "android")]
mod android {

    use irondash_run_loop::RunLoop;
    use once_cell::sync::OnceCell;

    use crate::init;

    pub static JAVA_VM: OnceCell<jni::JavaVM> = OnceCell::new();
    pub static CONTEXT: OnceCell<jni::objects::GlobalRef> = OnceCell::new();

    // JNI class loader can't load our classes, so we store the data util instance;
    // If there were more classes to load we could store the class loader instead
    pub static CLIP_DATA_HELPER: OnceCell<jni::objects::GlobalRef> = OnceCell::new();
    pub static DRAG_DROP_HELPER: OnceCell<jni::objects::GlobalRef> = OnceCell::new();

    #[no_mangle]
    #[allow(non_snake_case)]
    pub extern "C" fn Java_com_superlist_super_1native_1extensions_SuperNativeExtensionsPlugin_init(
        env: jni::JNIEnv,
        _class: jni::objects::JClass,
        context: jni::objects::JObject,
        clip_data_helper: jni::objects::JObject,
        drag_drop_helper: jni::objects::JObject,
    ) {
        use ::log::Level;
        use android_logger::Config;

        // This is to ensure that engine context is not used for sending things
        // to main thread. EngineContext main thread sender does not work properly
        // with RunLoop::poll_once, which is used during clipboard access.
        // Without this clipboard access may deadlock.
        RunLoop::set_main_thread();

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
        CLIP_DATA_HELPER.get_or_init(|| {
            env.new_global_ref(clip_data_helper)
                .expect("Failed to store clip data util")
        });
        DRAG_DROP_HELPER.get_or_init(|| {
            env.new_global_ref(drag_drop_helper)
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
    irondash_init_message_channel_context(data)
}
