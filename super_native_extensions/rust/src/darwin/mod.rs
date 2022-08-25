#![allow(clippy::let_unit_value)]

#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod os;

#[cfg(target_os = "ios")]
#[path = "ios/mod.rs"]
mod os;

pub use os::*;

#[allow(dead_code)]
mod common;

mod progress_bridge;
