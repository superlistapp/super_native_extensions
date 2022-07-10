#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod os;

#[cfg(target_os = "ios")]
#[path = "ios/mod.rs"]
mod os;

pub use os::*;

mod common;
