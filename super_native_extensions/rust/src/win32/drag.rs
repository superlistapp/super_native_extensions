use std::{
    rc::{Rc, Weak},
    sync::Arc,
};

use windows::Win32::Foundation::HWND;

use crate::{
    drag_manager::{DragRequest, PlatformDragContextDelegate},
    error::NativeExtensionsResult,
    util::DropNotifier,
};

use super::PlatformDataSource;

pub struct PlatformDragContext {
    id: i64,
    view: HWND,
}

impl PlatformDragContext {
    pub fn new(id: i64, view_handle: i64, delegate: Weak<dyn PlatformDragContextDelegate>) -> Self {
        Self {
            id: id,
            view: HWND(view_handle as isize),
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {}

    pub async fn start_drag(
        &self,
        request: DragRequest,
        data_source: Rc<PlatformDataSource>,
        drop_notifier: Arc<DropNotifier>,
    ) -> NativeExtensionsResult<()> {
        Ok(())
    }
}
