use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
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
            WindowsAndMessaging::{GetCursorPos, MK_LBUTTON},
        },
    },
};

use crate::{
    api_model::{DataProviderId, DragRequest, DropOperation, Point},
    drag_manager::{DataProviderEntry, DragSessionId, PlatformDragContextDelegate, PlatformDragContextId},
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    platform_impl::platform::data_object::DataObject,
};

use super::{
    common::{create_instance, image_data_to_hbitmap},
    data_object::DataObjectExt,
    drag_common::DropOperationExt,
};

pub struct PlatformDragContext {
    id: PlatformDragContextId,
    _view: HWND,
    delegate: Weak<dyn PlatformDragContextDelegate>,
    weak_self: Late<Weak<Self>>,
}

#[implement(IDropSource)]
pub struct DropSource {
    platform_context: Weak<PlatformDragContext>,
    last_reported_location: RefCell<Point>,
    session_id: DragSessionId,
    cancelled: Rc<Cell<bool>>,
}

#[allow(non_snake_case)]
impl DropSource {
    pub fn create(
        platform_context: Weak<PlatformDragContext>,
        session_id: DragSessionId,
        cancelled: Rc<Cell<bool>>,
    ) -> IDropSource {
        Self {
            platform_context,
            session_id,
            last_reported_location: RefCell::new(Point::default()),
            cancelled,
        }
        .into()
    }
}

impl IDropSource_Impl for DropSource {
    fn QueryContinueDrag(
        &self,
        fescapepressed: BOOL,
        grfkeystate: u32,
    ) -> windows::core::Result<()> {
        if fescapepressed.as_bool() {
            self.cancelled.replace(true);
            Err(DRAGDROP_S_CANCEL.into())
        } else if grfkeystate & MK_LBUTTON as u32 == 0 {
            Err(DRAGDROP_S_DROP.into())
        } else {
            let mut cursor_pos = POINT::default();
            unsafe { GetCursorPos(&mut cursor_pos as *mut _) };
            if let Some(context) = self.platform_context.upgrade() {
                if let Some(delegate) = context.delegate.upgrade() {
                    let location = Point {
                        x: cursor_pos.x as f64,
                        y: cursor_pos.y as f64,
                    };
                    if *self.last_reported_location.borrow() != location {
                        delegate.drag_session_did_move_to_location(
                            context.id,
                            self.session_id,
                            location.clone(),
                        );
                        self.last_reported_location.replace(location);
                    }
                }
            }
            Ok(())
        }
    }

    fn GiveFeedback(&self, _dweffect: u32) -> windows::core::Result<()> {
        Err(DRAGDROP_S_USEDEFAULTCURSORS.into())
    }
}

impl PlatformDragContext {
    pub fn new(
        id: PlatformDragContextId,
        view_handle: i64,
        delegate: Weak<dyn PlatformDragContextDelegate>,
    ) -> Self {
        Self {
            id,
            _view: HWND(view_handle as isize),
            delegate,
            weak_self: Late::new(),
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    pub fn needs_combined_drag_image() -> bool {
        true
    }

    pub async fn start_drag(
        &self,
        request: DragRequest,
        providers: HashMap<DataProviderId, DataProviderEntry>,
        session_id: DragSessionId,
    ) -> NativeExtensionsResult<()> {
        let weak_self = self.weak_self.clone();
        Context::get()
            .run_loop()
            .schedule_next(move || {
                if let Some(this) = weak_self.upgrade() {
                    this._start_drag(request, providers, session_id).ok_log();
                }
            })
            .detach();
        Ok(())
    }

    pub fn _start_drag(
        &self,
        request: DragRequest,
        mut providers: HashMap<DataProviderId, DataProviderEntry>,
        session_id: DragSessionId,
    ) -> NativeExtensionsResult<()> {
        let providers: Vec<_> = request
            .configuration
            .items
            .iter()
            .map(|item| {
                let entry = providers
                    .remove(&item.data_provider_id)
                    .expect("Missing data provider entry");
                (entry.provider, entry.handle)
            })
            .collect();

        let drag_image = &request.combined_drag_image.ok_or_else(|| {
            NativeExtensionsError::OtherError("Missing combined drag image".into())
        })?;

        let data_object = DataObject::create(providers);
        let helper: IDragSourceHelper = create_instance(&CLSID_DragDropHelper)?;
        let hbitmap = image_data_to_hbitmap(&drag_image.image_data)?;
        let device_pixel_ratio = drag_image.image_data.device_pixel_ratio.unwrap_or(1.0);
        let point_in_rect = Point {
            x: (request.position.x - drag_image.source_rect.x) * device_pixel_ratio,
            y: (request.position.y - drag_image.source_rect.y) * device_pixel_ratio,
        };

        let mut image = SHDRAGIMAGE {
            sizeDragImage: SIZE {
                cx: drag_image.image_data.width,
                cy: drag_image.image_data.height,
            },
            ptOffset: POINT {
                x: point_in_rect.x as i32,
                y: point_in_rect.y as i32,
            },
            hbmpDragImage: hbitmap,
            crColorKey: 0xFFFFFFFF,
        };
        unsafe {
            helper.InitializeFromBitmap(&mut image as *mut _, data_object.clone())?;
        }

        let mut allowed_effects: u32 = 0;
        for operation in &request.configuration.allowed_operations {
            allowed_effects |= operation.to_platform();
        }

        if let Some(delegate) = self.delegate.upgrade() {
            delegate
                .get_platform_drop_context(self.id)?
                .local_drag_will_start(request.configuration)?;
        }

        let cancelled = Rc::new(Cell::new(false));
        let drop_source = DropSource::create(self.weak_self.clone(), session_id, cancelled.clone());
        let mut effects_out: u32 = 0;
        unsafe {
            let _ = DoDragDrop(
                data_object.clone(),
                drop_source,
                allowed_effects,
                &mut effects_out as *mut u32,
            );
        }
        // Data source might be still in use through IDataObjectAsyncCapability,
        // but we want to let user know that drag session ended immediately.
        // COM will make sure that the data object is kept alive and when
        // deallocated we will get notification from drop notifier
        let effect = data_object.performed_drop_effect().unwrap_or(effects_out);
        if let Some(delegate) = self.delegate.upgrade() {
            delegate
                .get_platform_drop_context(self.id)?
                .local_drag_did_end()?;

            let operation = DropOperation::from_platform(effect);
            let operation = if operation == DropOperation::None && cancelled.get() {
                DropOperation::UserCancelled
            } else {
                operation
            };
            delegate.drag_session_did_end_with_operation(self.id, session_id, operation);
        }

        Ok(())
    }
}
