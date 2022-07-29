use std::{collections::HashMap, rc::Weak};

use crate::{
    api_model::{DataProviderId, DragRequest},
    drag_manager::{DataProviderEntry, DragSessionId, PlatformDragContextDelegate},
    error::NativeExtensionsResult,
};

pub struct PlatformDragContext {}

impl PlatformDragContext {
    pub fn new(id: i64, view_handle: i64, delegate: Weak<dyn PlatformDragContextDelegate>) -> Self {
        Self {}
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {}

    pub fn needs_combined_drag_image() -> bool {
        true
    }

    pub async fn start_drag(
        &self,
        request: DragRequest,
        providers: HashMap<DataProviderId, DataProviderEntry>,
        session_id: DragSessionId,
    ) -> NativeExtensionsResult<()> {
        Ok(())
    }
}
