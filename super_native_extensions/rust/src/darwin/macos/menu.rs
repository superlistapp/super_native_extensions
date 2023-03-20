use std::rc::{Rc, Weak};

use irondash_message_channel::IsolateId;

use crate::{
    api_model::Menu,
    error::NativeExtensionsResult,
    menu_manager::{PlatformMenuContextDelegate, PlatformMenuContextId, PlatformMenuDelegate},
};

pub struct PlatformMenuContext {}

#[derive(Debug)]
pub struct PlatformMenu {}

impl PlatformMenu {
    pub fn new(
        _isolate: IsolateId,
        _delegate: Weak<dyn PlatformMenuDelegate>,
        _menu: Menu,
    ) -> NativeExtensionsResult<Rc<Self>> {
        Ok(Rc::new(Self {}))
    }
}

impl PlatformMenuContext {
    pub fn new(
        _id: PlatformMenuContextId,
        _engine_handle: i64,
        _delegate: Weak<dyn PlatformMenuContextDelegate>,
    ) -> NativeExtensionsResult<Self> {
        Ok(Self {})
    }

    pub fn assign_weak_self(&self, _weak_self: Weak<Self>) {}
}
