use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    sync::Arc,
};

use nativeshell_core::{platform::run_loop::PollSession, util::Late, Context, Value};
use windows::{
    core::{implement, Interface, PCWSTR},
    Win32::{
        Foundation::{E_OUTOFMEMORY, HWND, POINT, POINTL, S_OK},
        Graphics::Gdi::ScreenToClient,
        System::{
            Com::IDataObject,
            LibraryLoader::GetModuleHandleW,
            Ole::{IDropTarget, IDropTarget_Impl, RegisterDragDrop, RevokeDragDrop},
            Threading::{GetCurrentProcessId, GetCurrentThreadId},
        },
        UI::{
            Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK},
            Shell::{CLSID_DragDropHelper, IDataObjectAsyncCapability, IDropTargetHelper},
            WindowsAndMessaging::{EVENT_OBJECT_DESTROY, OBJID_WINDOW, WINEVENT_INCONTEXT},
        },
    },
};

use crate::{
    api_model::{DragConfiguration, DropOperation, Point},
    drop_manager::{
        BaseDropEvent, DropEvent, DropItem, DropSessionId, PlatformDropContextDelegate,
        PlatformDropContextId,
    },
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    reader_manager::RegisteredDataReader,
    util::{DropNotifier, NextId},
};

use super::{
    common::{create_instance, get_dpi_for_window},
    drag_common::DropOperationExt,
    PlatformDataReader,
};

pub struct PlatformDropContext {
    id: PlatformDropContextId,
    weak_self: Late<Weak<Self>>,
    view: HWND,
    delegate: Weak<dyn PlatformDropContextDelegate>,
    hook: Late<HWINEVENTHOOK>,
    local_session: RefCell<Option<DragConfiguration>>,
    next_session_id: Cell<i64>,
    current_session: RefCell<Option<Rc<Session>>>,
}

thread_local! {
    static HOOK_TO_HWND: RefCell<HashMap<isize, HWND>> = RefCell::new(HashMap::new());
}

struct Session {
    id: DropSessionId,
    is_inside: Cell<bool>,
    data_object: IDataObject,
    last_operation: Cell<DropOperation>,
    async_result: Rc<Cell<Option<u32>>>,
    reader: Rc<PlatformDataReader>,
    registered_reader: RegisteredDataReader,
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
            local_session: RefCell::new(None),
            next_session_id: Cell::new(0),
            current_session: RefCell::new(None),
        }
    }

    pub fn register_drop_formats(&self, _formats: &[String]) -> NativeExtensionsResult<()> {
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
            if let Err(err) = RegisterDragDrop(self.view, target) {
                if err.code() == E_OUTOFMEMORY {
                    eprintln!("**");
                    eprintln!("** RegisterDragDrop failed: ");
                    eprintln!(
                        "** Please use OleInitialize instead of CoInitializeEx to initialize COM."
                    );
                    eprintln!("**");
                }
                Result::<(), _>::Err(err).ok_log();
            }

            // Unregistering in drop is too late as the HWND is already destroyed.
            // Instead we setup hook for OBJECT_DESTROY and revoke drop target there.
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

    pub fn local_drag_will_start(
        &self,
        configuration: DragConfiguration,
    ) -> NativeExtensionsResult<()> {
        self.local_session.replace(Some(configuration));
        Ok(())
    }

    pub fn local_drag_did_end(&self) -> NativeExtensionsResult<()> {
        self.local_session.replace(None);
        if self.current_session.borrow().is_some() {
            self.drop_end()?;
        }
        Ok(())
    }

    pub fn get_local_drag_data(&self) -> NativeExtensionsResult<Option<Vec<Value>>> {
        Ok(self
            .local_session
            .borrow()
            .as_ref()
            .map(|s| s.get_local_data()))
    }

    fn delegate(&self) -> NativeExtensionsResult<Rc<dyn PlatformDropContextDelegate>> {
        self.delegate
            .upgrade()
            .ok_or_else(|| NativeExtensionsError::OtherError("missing context delegate".into()))
    }

    fn drop_exit(&self) -> NativeExtensionsResult<()> {
        if let Some(session) = self.current_session.borrow().as_ref().cloned() {
            self.delegate()?.send_drop_leave(
                self.id,
                BaseDropEvent {
                    session_id: session.id,
                },
            );
        }
        Ok(())
    }

    fn drop_end(&self) -> NativeExtensionsResult<()> {
        if let Some(session) = self.current_session.borrow_mut().take() {
            self.delegate()?.send_drop_ended(
                self.id,
                BaseDropEvent {
                    session_id: session.id,
                },
            );
        }
        Ok(())
    }

    fn event_for_session(
        &self,
        session: &Rc<Session>,
        pt: &POINTL,
        _grfkeystate: u32,
        mask: u32,
        accepted_operation: Option<DropOperation>,
    ) -> NativeExtensionsResult<DropEvent> {
        let local_data: Vec<_> = self
            .local_session
            .borrow()
            .as_ref()
            .map(|a| a.items.iter().map(|i| i.local_data.clone()).collect())
            .unwrap_or_default();

        let mut pt = POINT { x: pt.x, y: pt.y };
        unsafe {
            ScreenToClient(self.view, &mut pt as *mut _);
        }
        let scaling = get_dpi_for_window(self.view) as f64 / 96.0;

        let reader_items = session.reader.get_items_sync()?;

        let items: Vec<_> = (0..local_data.len().max(reader_items.len()))
            .map(|index| {
                Ok(DropItem {
                    item_id: (index as i64).into(),
                    formats: match reader_items.get(index) {
                        Some(item) => session.reader.get_formats_for_item_sync(*item)?,
                        None => Vec::new(),
                    },
                    local_data: local_data.get(index).cloned().unwrap_or(Value::Null),
                })
            })
            .collect::<NativeExtensionsResult<_>>()?;

        Ok(DropEvent {
            session_id: session.id,
            location_in_view: Point {
                x: pt.x as f64 / scaling,
                y: pt.y as f64 / scaling,
            },
            allowed_operations: DropOperation::from_platform_mask(mask),
            accepted_operation,
            items,
            reader: Some(session.registered_reader.clone()),
        })
    }

    fn on_drag_enter(
        &self,
        pdataobj: &Option<IDataObject>,
        grfkeystate: u32,
        pt: &POINTL,
        pdweffect: *mut u32,
    ) -> NativeExtensionsResult<()> {
        if self.current_session.borrow().is_some() && self.local_session.borrow().is_none() {
            // shouldn't happen
            if self
                .current_session
                .borrow()
                .as_ref()
                .unwrap()
                .is_inside
                .get()
            {
                self.drop_exit()?;
            }
            self.drop_end()?;
        }
        let effect = unsafe { &mut *pdweffect };
        if let Some(data_object) = pdataobj {
            let delegate = self.delegate()?;
            let session = self
                .current_session
                .borrow_mut()
                .get_or_insert_with(|| {
                    let async_result = Rc::new(Cell::new(None));
                    let data_object_clone = data_object.clone();
                    let async_result_clone = async_result.clone();
                    // Drop notifier invoked when reader gets destroyed. If we started
                    // async operation on data object this will end it.
                    let drop_notifier = Arc::new(DropNotifier::new(move || {
                        if let Some(res) = async_result_clone.get().take() {
                            if let Ok(data_object_async) =
                                data_object_clone.cast::<IDataObjectAsyncCapability>()
                            {
                                unsafe {
                                    data_object_async.EndOperation(S_OK, None, res).ok_log();
                                }
                            }
                        }
                    }));
                    let reader = PlatformDataReader::new_with_data_object(
                        data_object.clone(),
                        Some(drop_notifier),
                    );
                    let registered_reader =
                        delegate.register_platform_reader(self.id, reader.clone());
                    Rc::new(Session {
                        id: self.next_session_id.next_id().into(),
                        is_inside: Cell::new(true),
                        data_object: data_object.clone(),
                        last_operation: Cell::new(DropOperation::None),
                        async_result,
                        reader,
                        registered_reader,
                    })
                })
                .clone();
            session.is_inside.set(true);
            let session_clone = session.clone();
            let event = self.event_for_session(&session, pt, grfkeystate, *effect, None)?;
            delegate.send_drop_update(
                self.id,
                event,
                Box::new(move |res| {
                    let res = res.ok_log().unwrap_or(DropOperation::None);
                    session_clone.last_operation.set(res);
                }),
            );
            *effect = session.last_operation.get().to_platform();
        } else {
            *effect = 0;
        }

        Ok(())
    }

    fn on_drag_over(
        &self,
        grfkeystate: u32,
        pt: &POINTL,
        pdweffect: *mut u32,
    ) -> NativeExtensionsResult<()> {
        let effect = unsafe { &mut *pdweffect };
        if let Some(session) = self.current_session.borrow().as_ref().cloned() {
            let session_clone = session.clone();
            let event = self.event_for_session(&session, pt, grfkeystate, *effect, None)?;
            self.delegate()?.send_drop_update(
                self.id,
                event,
                Box::new(move |res| {
                    let res = res.ok_log().unwrap_or(DropOperation::None);
                    session_clone.last_operation.set(res);
                }),
            );
            *effect = session.last_operation.get().to_platform();
        } else {
            *effect = 0;
        }
        Ok(())
    }

    fn on_drag_leave(&self) -> NativeExtensionsResult<()> {
        self.drop_exit()?;
        if let Some(s) = self.current_session.borrow_mut().as_ref() {
            s.is_inside.set(false)
        }
        if self.local_session.borrow().is_none() {
            self.drop_end()?;
        }
        Ok(())
    }

    fn on_drop(
        &self,
        _pdataobj: &Option<IDataObject>,
        grfkeystate: u32,
        pt: &POINTL,
        pdweffect: *mut u32,
    ) -> NativeExtensionsResult<()> {
        let effect = unsafe { &mut *pdweffect };
        let session = self.current_session.borrow().as_ref().cloned();
        if let Some(session) = session {
            *effect = session.last_operation.get().to_platform();
            let event = self.event_for_session(
                &session,
                pt,
                grfkeystate,
                *effect,
                Some(session.last_operation.get()),
            )?;
            let done = Rc::new(Cell::new(false));
            let done_clone = done.clone();
            self.delegate()?.send_perform_drop(
                self.id,
                event,
                Box::new(move |r| {
                    r.ok_log();
                    done_clone.set(true);
                }),
            );
            let data_object_async = session.data_object.cast::<IDataObjectAsyncCapability>();
            if let Ok(data_object_async) = data_object_async {
                if let Ok(res) = unsafe { data_object_async.GetAsyncMode() } {
                    if res.as_bool() {
                        // this will be read by drop notifier in DataReader and used for
                        // IDataObjectAsyncCapability::EndOperation result (when data reader gets dropped)
                        session.async_result.set(Some(*effect));
                        unsafe {
                            data_object_async.StartOperation(None).ok_log();
                        }
                    }
                }
            }
            let mut poll_session = PollSession::new();
            while !done.get() {
                Context::get()
                    .run_loop()
                    .platform_run_loop
                    .poll_once(&mut poll_session);
            }
            self.drop_end()?;
        } else {
            *effect = 0;
        }
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
        if let Some(context) = self.platform_context.upgrade() {
            context
                .on_drag_enter(pdataobj, grfkeystate, pt, pdweffect)
                .ok_log();
        }
        Ok(())
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
        if let Some(context) = self.platform_context.upgrade() {
            context.on_drag_over(grfkeystate, pt, pdweffect).ok_log();
        }
        Ok(())
    }

    fn DragLeave(&self) -> windows::core::Result<()> {
        if let Some(drop_target_helper) = &self.drop_target_helper {
            unsafe {
                drop_target_helper.DragLeave().ok_log();
            }
        }
        if let Some(context) = self.platform_context.upgrade() {
            context.on_drag_leave().ok_log();
        }
        Ok(())
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
        if let Some(context) = self.platform_context.upgrade() {
            context
                .on_drop(pdataobj, grfkeystate, pt, pdweffect)
                .ok_log();
        }
        Ok(())
    }
}
