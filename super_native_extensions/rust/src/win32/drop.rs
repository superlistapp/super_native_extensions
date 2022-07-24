use std::{cell::RefCell, collections::HashMap, rc::Weak};

use nativeshell_core::util::Late;
use windows::{
    core::{implement, PCWSTR},
    Win32::{
        Foundation::{HWND, POINT, POINTL},
        System::{
            Com::IDataObject,
            LibraryLoader::GetModuleHandleW,
            Ole::{IDropTarget, IDropTarget_Impl, RegisterDragDrop, RevokeDragDrop},
            Threading::{GetCurrentProcessId, GetCurrentThreadId},
        },
        UI::{
            Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK},
            Shell::{CLSID_DragDropHelper, IDropTargetHelper},
            WindowsAndMessaging::{EVENT_OBJECT_DESTROY, WINEVENT_INCONTEXT, OBJID_WINDOW},
        },
    },
};

use crate::{
    drop_manager::{PlatformDropContextDelegate, PlatformDropContextId},
    error::NativeExtensionsResult,
    log::OkLog,
};

use super::common::create_instance;

pub struct PlatformDropContext {
    id: PlatformDropContextId,
    weak_self: Late<Weak<Self>>,
    view: HWND,
    delegate: Weak<dyn PlatformDropContextDelegate>,
    hook: Late<HWINEVENTHOOK>,
}

thread_local! {
    static HOOK_TO_HWND: RefCell<HashMap<isize, HWND>> = RefCell::new(HashMap::new());
}

impl PlatformDropContext {
    pub fn new(
        id: PlatformDropContextId,
        view_handle: i64,
        delegate: Weak<dyn PlatformDropContextDelegate>,
    ) -> Self {
        Self {
            id,
            weak_self: Late::new(),
            view: HWND(view_handle as isize),
            delegate,
            hook: Late::new(),
        }
    }

    pub fn register_drop_types(&self, _types: &[String]) -> NativeExtensionsResult<()> {
        Ok(())
    }

    unsafe extern "system" fn hook_procfn(
        hwineventhook: HWINEVENTHOOK,
        _event: u32,
        hwnd: HWND,
        idobject: i32,
        _idchild: i32,
        _ideventthread: u32,
        _dwmseventtime: u32,
    ) {
        if idobject != OBJID_WINDOW.0 {
            return;
        }
        let hook_hwnd = HOOK_TO_HWND.with(|a| a.borrow().get(&hwineventhook.0).cloned());
        if let Some(hook_hwnd) = hook_hwnd {
            if hook_hwnd == hwnd {
                RevokeDragDrop(hook_hwnd).ok_log();
            }
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self.clone());
        let target: IDropTarget = DropTarget::new(self.view, weak_self).into();
        unsafe {
            RegisterDragDrop(self.view, target).ok_log();

            // Unregistering in Drop is too late as the HWND is already destroyed.
            // Set we setup hook for OBJECT_DESTROY and revoke drop target there.
            let hook = SetWinEventHook(
                EVENT_OBJECT_DESTROY,
                EVENT_OBJECT_DESTROY,
                GetModuleHandleW(PCWSTR(std::ptr::null_mut())).unwrap(),
                Some(Self::hook_procfn),
                GetCurrentProcessId(),
                GetCurrentThreadId(),
                WINEVENT_INCONTEXT,
            );
            self.hook.set(hook);
            HOOK_TO_HWND.with(|a| a.borrow_mut().insert(hook.0, self.view));
        }
    }

    fn on_drag_enter(
        &self,
        pdataobj: &Option<IDataObject>,
        grfkeystate: u32,
        pt: &POINTL,
        pdweffect: *mut u32,
    ) -> NativeExtensionsResult<()> {
        Ok(())
    }

    fn on_drag_over(
        &self,
        grfkeystate: u32,
        pt: &POINTL,
        pdweffect: *mut u32,
    ) -> NativeExtensionsResult<()> {
        Ok(())
    }

    fn on_drag_leave(&self) -> NativeExtensionsResult<()> {
        Ok(())
    }

    fn on_drop(
        &self,
        pdataobj: &Option<IDataObject>,
        grfkeystate: u32,
        pt: &POINTL,
        pdweffect: *mut u32,
    ) -> NativeExtensionsResult<()> {
        Ok(())
    }
}

impl Drop for PlatformDropContext {
    fn drop(&mut self) {
        unsafe {
            let hook = *self.hook;
            UnhookWinEvent(hook);
            HOOK_TO_HWND.with(|a| a.borrow_mut().remove(&hook.0));
        }
    }
}

#[implement(IDropTarget)]
struct DropTarget {
    hwnd: HWND,
    platform_context: Weak<PlatformDropContext>,
    drop_target_helper: Option<IDropTargetHelper>,
}

impl DropTarget {
    fn new(hwnd: HWND, platform_context: Weak<PlatformDropContext>) -> Self {
        Self {
            hwnd,
            platform_context,
            drop_target_helper: create_instance(&CLSID_DragDropHelper).ok_log(),
        }
    }
}

impl IDropTarget_Impl for DropTarget {
    fn DragEnter(
        &self,
        pdataobj: &Option<IDataObject>,
        grfkeystate: u32,
        pt: &POINTL,
        pdweffect: *mut u32,
    ) -> windows::core::Result<()> {
        if let Some(drop_target_helper) = &self.drop_target_helper {
            unsafe {
                drop_target_helper
                    .DragEnter(
                        self.hwnd,
                        pdataobj,
                        pt as *const POINTL as *const _,
                        *pdweffect,
                    )
                    .ok_log();
            }
        }
        match self.platform_context.upgrade() {
            Some(context) => context
                .on_drag_enter(pdataobj, grfkeystate, pt, pdweffect)
                .map_err(|e| e.into()),
            None => Ok(()),
        }
    }

    fn DragOver(
        &self,
        grfkeystate: u32,
        pt: &POINTL,
        pdweffect: *mut u32,
    ) -> windows::core::Result<()> {
        if let Some(drop_target_helper) = &self.drop_target_helper {
            unsafe {
                drop_target_helper
                    .DragOver(pt as *const POINTL as *const _, *pdweffect)
                    .ok_log();
            }
        }
        match self.platform_context.upgrade() {
            Some(context) => context
                .on_drag_over(grfkeystate, pt, pdweffect)
                .map_err(|e| e.into()),
            None => Ok(()),
        }
    }

    fn DragLeave(&self) -> windows::core::Result<()> {
        if let Some(drop_target_helper) = &self.drop_target_helper {
            unsafe {
                drop_target_helper.DragLeave().ok_log();
            }
        }
        match self.platform_context.upgrade() {
            Some(context) => context.on_drag_leave().map_err(|e| e.into()),
            None => Ok(()),
        }
    }

    fn Drop(
        &self,
        pdataobj: &Option<IDataObject>,
        grfkeystate: u32,
        pt: &POINTL,
        pdweffect: *mut u32,
    ) -> windows::core::Result<()> {
        if let Some(drop_target_helper) = &self.drop_target_helper {
            unsafe {
                drop_target_helper
                    .Drop(pdataobj, pt as *const POINTL as *const _, *pdweffect)
                    .ok_log();
            }
        }
        match self.platform_context.upgrade() {
            Some(context) => context
                .on_drop(pdataobj, grfkeystate, pt, pdweffect)
                .map_err(|e| e.into()),
            None => Ok(()),
        }
    }
}
