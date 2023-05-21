use std::rc::{Rc, Weak};

use irondash_engine_context::EngineContext;
use irondash_message_channel::IsolateId;

use crate::{
    api_model::{ImageData, Menu, ShowContextMenuRequest, ShowContextMenuResponse},
    error::{NativeExtensionsError, NativeExtensionsResult},
    menu_manager::{PlatformMenuContextDelegate, PlatformMenuContextId, PlatformMenuDelegate},
};

pub struct PlatformMenuContext {
    _delegate: Weak<dyn PlatformMenuContextDelegate>,
    _view: isize,
}

pub struct PlatformMenu {}

impl std::fmt::Debug for PlatformMenu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformMenu").finish()
    }
}

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
        engine_handle: i64,
        delegate: Weak<dyn PlatformMenuContextDelegate>,
    ) -> NativeExtensionsResult<Self> {
        let view = EngineContext::get()?.get_flutter_view(engine_handle)?;
        Ok(Self {
            _delegate: delegate,
            _view: view,
        })
    }

    pub fn assign_weak_self(&self, _weak_self: Weak<Self>) {}

    pub fn update_preview_image(
        &self,
        _configuration_id: i64,
        _image_data: ImageData,
    ) -> NativeExtensionsResult<()> {
        Err(NativeExtensionsError::UnsupportedOperation)
    }

    pub async fn show_context_menu(
        &self,
        _request: ShowContextMenuRequest,
    ) -> NativeExtensionsResult<ShowContextMenuResponse> {
        Err(NativeExtensionsError::UnsupportedOperation)
    }
}
