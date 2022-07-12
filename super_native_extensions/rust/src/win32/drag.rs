use std::{
    rc::{Rc, Weak},
    sync::Arc,
};

use nativeshell_core::{util::Late, Context};
use windows::{
    core::implement,
    Win32::{
        Foundation::{
            BOOL, DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS, HWND, POINT,
            SIZE,
        },
        System::Ole::{DoDragDrop, IDropSource, IDropSource_Impl},
        UI::{
            Shell::{CLSID_DragDropHelper, IDragSourceHelper, SHDRAGIMAGE},
            WindowsAndMessaging::MK_LBUTTON,
        },
    },
};

use crate::{
    api_model::{DragRequest, DropOperation},
    drag_manager::{DragSessionId, PlatformDragContextDelegate},
    error::NativeExtensionsResult,
    log::OkLog,
    platform_impl::platform::{common::get_dpi_for_window, data_object::DataObject},
    util::DropNotifier,
};

use super::{
    common::{create_instance, image_data_to_hbitmap},
    data_object::DataObjectExt,
    drag_common::DropOperationExt,
    PlatformDataSource,
};

pub struct PlatformDragContext {
    id: i64,
    view: HWND,
    delegate: Weak<dyn PlatformDragContextDelegate>,
    weak_self: Late<Weak<Self>>,
}

#[implement(IDropSource)]
pub struct DropSource {}

#[allow(non_snake_case)]
impl DropSource {
    pub fn create() -> IDropSource {
        Self {}.into()
    }
}

impl IDropSource_Impl for DropSource {
    fn QueryContinueDrag(
        &self,
        fescapepressed: BOOL,
        grfkeystate: u32,
    ) -> windows::core::Result<()> {
        if fescapepressed.as_bool() {
            Err(DRAGDROP_S_CANCEL.into())
        } else if grfkeystate & MK_LBUTTON as u32 == 0 {
            Err(DRAGDROP_S_DROP.into())
        } else {
            Ok(())
        }
    }

    fn GiveFeedback(&self, _dweffect: u32) -> windows::core::Result<()> {
        Err(DRAGDROP_S_USEDEFAULTCURSORS.into())
    }
}

impl PlatformDragContext {
    pub fn new(id: i64, view_handle: i64, delegate: Weak<dyn PlatformDragContextDelegate>) -> Self {
        Self {
            id,
            view: HWND(view_handle as isize),
            delegate,
            weak_self: Late::new(),
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    pub async fn start_drag(
        &self,
        request: DragRequest,
        data_source: Rc<PlatformDataSource>,
        drop_notifier: Arc<DropNotifier>,
        session_id: DragSessionId,
    ) -> NativeExtensionsResult<()> {
        let weak_self = self.weak_self.clone();
        Context::get()
            .run_loop()
            .schedule_next(move || {
                if let Some(this) = weak_self.upgrade() {
                    this._start_drag(request, data_source, drop_notifier, session_id)
                        .ok_log();
                }
            })
            .detach();
        Ok(())
    }

    pub fn _start_drag(
        &self,
        request: DragRequest,
        data_source: Rc<PlatformDataSource>,
        drop_notifier: Arc<DropNotifier>,
        session_id: DragSessionId,
    ) -> NativeExtensionsResult<()> {
        let data_object = DataObject::create(data_source, drop_notifier);
        let helper: IDragSourceHelper = create_instance(&CLSID_DragDropHelper)?;
        let drag_image = request.drag_data.drag_image;
        let hbitmap = image_data_to_hbitmap(&drag_image.image_data)?;
        let scaling = get_dpi_for_window(self.view) as f64 / 96.0;

        let mut image = SHDRAGIMAGE {
            sizeDragImage: SIZE {
                cx: drag_image.image_data.width,
                cy: drag_image.image_data.height,
            },
            ptOffset: POINT {
                x: (drag_image.point_in_rect.x * scaling) as i32,
                y: (drag_image.point_in_rect.y * scaling) as i32,
            },
            hbmpDragImage: hbitmap,
            crColorKey: 0xFFFFFFFF,
        };
        unsafe {
            helper.InitializeFromBitmap(&mut image as *mut _, data_object.clone())?;
        }
        let drop_source = DropSource::create();
        unsafe {
            let mut _effects_out: u32 = 0;
            let mut allowed_effects: u32 = 0;
            for operation in request.drag_data.allowed_operations {
                allowed_effects |= operation.to_platform();
            }
            let _ = DoDragDrop(
                data_object.clone(),
                drop_source,
                allowed_effects,
                &mut _effects_out as *mut u32,
            );
        }
        // Data source might be still in use through IDataObjectAsyncCapability,
        // but we want to let user know that drag session ended immediately.
        // COM will make sure that the data object is kept alive and when
        // deallocated we will get notification from drop notifier
        let effect = data_object.performed_drop_effect().unwrap_or(0);
        if let Some(delegate) = self.delegate.upgrade() {
            let operation = DropOperation::from_platform(effect);
            delegate.drag_session_did_end_with_operation(self.id, session_id, operation);
        }

        Ok(())
    }
}
