use std::rc::Weak;

use crate::{
    error::{NativeExtensionsError, NativeExtensionsResult},
    hot_key_manager::{HotKeyCreateRequest, HotKeyHandle, HotKeyManagerDelegate},
};

pub struct PlatformHotKeyManager {}

impl PlatformHotKeyManager {
    pub fn new(_delegate: Weak<dyn HotKeyManagerDelegate>) -> Self {
        Self {}
    }

    pub fn assign_weak_self(&self, _weak: Weak<PlatformHotKeyManager>) {}

    pub fn create_hot_key(
        &self,
        _handle: HotKeyHandle,
        _request: HotKeyCreateRequest,
    ) -> NativeExtensionsResult<()> {
        Err(NativeExtensionsError::UnsupportedOperation)
    }

    pub fn destroy_hot_key(&self, _handle: HotKeyHandle) -> NativeExtensionsResult<()> {
        Err(NativeExtensionsError::UnsupportedOperation)
    }
}
