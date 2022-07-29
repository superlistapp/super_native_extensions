use std::rc::Weak;

use crate::{drop_manager::PlatformDropContextDelegate, error::NativeExtensionsResult};

pub struct PlatformDropContext {}

impl PlatformDropContext {
    pub fn new(id: i64, view_handle: i64, delegate: Weak<dyn PlatformDropContextDelegate>) -> Self {
        Self {}
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {}

    pub fn register_drop_types(&self, types: &[String]) -> NativeExtensionsResult<()> {
        Ok(())
    }
}
