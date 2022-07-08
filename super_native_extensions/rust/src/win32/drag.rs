use std::{
    rc::{Rc, Weak},
    sync::Arc,
    thread,
};

use windows::{
    core::implement,
    Win32::{
        Foundation::{
            BOOL, DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS, HWND, POINT,
            SIZE,
        },
        System::Ole::{
            DoDragDrop, IDropSource, IDropSource_Impl, DROPEFFECT_COPY, DROPEFFECT_MOVE,
        },
        UI::{
            Shell::{CLSID_DragDropHelper, IDragSourceHelper, SHDRAGIMAGE},
            WindowsAndMessaging::MK_LBUTTON,
        },
    },
};

use crate::{
    drag_manager::{DragRequest, PlatformDragContextDelegate},
    error::NativeExtensionsResult,
    util::DropNotifier, platform_impl::platform::data_object::DataObject,
};

use super::{
    common::{create_instance, image_data_to_hbitmap},
    PlatformDataSource,
};

pub struct PlatformDragContext {
    id: i64,
    view: HWND,
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

    fn GiveFeedback(&self, dweffect: u32) -> windows::core::Result<()> {
        Err(DRAGDROP_S_USEDEFAULTCURSORS.into())
    }
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
        let data_object = DataObject::create(data_source, drop_notifier);
        let helper: IDragSourceHelper = create_instance(&CLSID_DragDropHelper)?;
        let hbitmap = image_data_to_hbitmap(&request.image)?;
        let mut image = SHDRAGIMAGE {
            sizeDragImage: SIZE {
                cx: request.image.width,
                cy: request.image.height,
            },
            ptOffset: POINT {
                // x: cursor_pos.x - image_start.x,
                // y: cursor_pos.y - image_start.y,
                x: 0,
                y: 0,
            },
            hbmpDragImage: hbitmap,
            crColorKey: 0xFFFFFFFF,
        };
        unsafe {
            helper.InitializeFromBitmap(&mut image as *mut _, data_object.clone())?;
        }
        println!("DDD BEGIN {:?}", thread::current().id());
        let drop_source = DropSource::create();
        unsafe {
            let mut effects_out: u32 = 0;
            let res = DoDragDrop(
                data_object,
                drop_source,
                DROPEFFECT_COPY | DROPEFFECT_MOVE,
                &mut effects_out as *mut u32,
            );
            println!("DDD END {:?}", res);
        }

        Ok(())
    }
}
