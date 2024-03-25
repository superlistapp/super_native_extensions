use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    os::raw::c_void,
    rc::{Rc, Weak},
    slice,
    sync::Arc,
};

use async_trait::async_trait;
use irondash_message_channel::{
    AsyncMethodHandler, AsyncMethodInvoker, IntoPlatformResult, IntoValue, IsolateId, Late,
    MethodCall, PlatformError, PlatformResult, RegisteredAsyncMethodHandler, TryFromValue, Value,
};
use irondash_run_loop::spawn;

use crate::{
    api_model::{DataProvider, DataProviderId, DataProviderValueId},
    context::Context,
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    platform_impl::platform::{platform_stream_close, platform_stream_write, PlatformDataProvider},
    util::{DropNotifier, NextId},
    value_promise::{ValuePromise, ValuePromiseResult, ValuePromiseSetCancel},
};

pub enum VirtualFileResult {
    Done,
    Error { message: String },
    Cancelled,
}

/// Keeps the virtual session alive
pub struct VirtualSessionHandle(DropNotifier);

#[allow(dead_code)]
impl VirtualSessionHandle {
    pub fn dispose(&self) {
        self.0.dispose();
    }
}

/// Keeps the data provider alive
#[allow(unused)] // DropNotifier is not read but needs to be retained.
pub struct DataProviderHandle(DropNotifier);

impl From<DropNotifier> for DataProviderHandle {
    fn from(notifier: DropNotifier) -> Self {
        DataProviderHandle(notifier)
    }
}

#[async_trait(?Send)]
pub trait PlatformDataProviderDelegate {
    fn get_lazy_data(
        &self,
        isolate_id: IsolateId,
        data_id: DataProviderValueId,
        on_done: Option<Box<dyn FnOnce()>>,
    ) -> Arc<ValuePromise>;

    async fn get_lazy_data_async(
        &self,
        isolate_id: IsolateId,
        data_id: DataProviderValueId,
    ) -> ValuePromiseResult;

    fn get_virtual_file(
        &self,
        isolate_id: IsolateId,
        virtual_file_id: DataProviderValueId,
        stream_handle: i32,
        on_size_known: Box<dyn Fn(Option<i64>)>,
        on_progress: Box<dyn Fn(f64 /* 0.0 - 1.0 */)>,
        on_done: Box<dyn FnOnce(VirtualFileResult)>,
    ) -> Arc<VirtualSessionHandle>;
}

pub struct DataProviderManager {
    weak_self: Late<Weak<Self>>,
    invoker: Late<AsyncMethodInvoker>,
    next_id: Cell<i64>,
    providers: RefCell<HashMap<DataProviderId, DataProviderEntry>>,
    virtual_sessions: RefCell<HashMap<VirtualSessionId, VirtualFileSession>>,
}

pub trait GetDataProviderManager {
    fn data_provider_manager(&self) -> Rc<DataProviderManager>;
}

impl GetDataProviderManager for Context {
    fn data_provider_manager(&self) -> Rc<DataProviderManager> {
        self.get_attachment(DataProviderManager::new).handler()
    }
}

struct DataProviderEntry {
    isolate_id: IsolateId,
    platform_data_provider: Rc<PlatformDataProvider>,
}

#[derive(Debug, TryFromValue, IntoValue, Clone, Copy, PartialEq, Hash, Eq)]
struct VirtualSessionId(i64);

impl From<i64> for VirtualSessionId {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

struct VirtualFileSession {
    isolate_id: IsolateId,
    size_known: Cell<bool>,
    on_size_known: Box<dyn Fn(Option<i64>)>,
    on_progress: Box<dyn Fn(f64 /* 0.0 - 1.0 */)>,
    on_done: Box<dyn FnOnce(VirtualFileResult)>,
}

impl DataProviderManager {
    pub fn new() -> RegisteredAsyncMethodHandler<Self> {
        Self {
            weak_self: Late::new(),
            invoker: Late::new(),
            next_id: Cell::new(1),
            providers: RefCell::new(HashMap::new()),
            virtual_sessions: RefCell::new(HashMap::new()),
        }
        .register("DataProviderManager")
    }

    pub fn get_platform_data_provider(
        &self,
        provider_id: DataProviderId,
    ) -> NativeExtensionsResult<Rc<PlatformDataProvider>> {
        self.providers
            .borrow()
            .get(&provider_id)
            .map(|e| e.platform_data_provider.clone())
            .ok_or(NativeExtensionsError::DataSourceNotFound)
    }

    fn register_provider(
        &self,
        source: DataProvider,
        isolate_id: IsolateId,
    ) -> NativeExtensionsResult<DataProviderId> {
        let platform_data_source = Rc::new(PlatformDataProvider::new(
            self.weak_self.clone(),
            isolate_id,
            source,
        ));
        let id = self.next_id.next_id().into();
        platform_data_source.assign_weak_self(Rc::downgrade(&platform_data_source));
        self.providers.borrow_mut().insert(
            id,
            DataProviderEntry {
                isolate_id,
                platform_data_provider: platform_data_source,
            },
        );
        Ok(id)
    }

    fn unregister_provider(&self, source: DataProviderId) -> NativeExtensionsResult<()> {
        self.providers.borrow_mut().remove(&source);
        Ok(())
    }

    fn virtual_file_update_progress(
        &self,
        progress: VirtualFileUpdateProgress,
    ) -> NativeExtensionsResult<()> {
        let sessions = self.virtual_sessions.borrow();
        let session = sessions
            .get(&progress.session_id)
            .ok_or(NativeExtensionsError::VirtualFileSessionNotFound)?;
        (session.on_progress)(progress.progress);
        Ok(())
    }

    fn virtual_file_size_known(
        &self,
        size_known: VirtualFileSizeKnown,
    ) -> NativeExtensionsResult<()> {
        let sessions = self.virtual_sessions.borrow();
        let session = sessions
            .get(&size_known.session_id)
            .ok_or(NativeExtensionsError::VirtualFileSessionNotFound)?;
        session.size_known.replace(true);
        (session.on_size_known)(Some(size_known.file_size));
        Ok(())
    }

    fn virtual_file_complete(&self, complete: VirtualFileComplete) -> NativeExtensionsResult<()> {
        let session = self
            .virtual_sessions
            .borrow_mut()
            .remove(&complete.session_id)
            .ok_or(NativeExtensionsError::VirtualFileSessionNotFound)?;
        if !session.size_known.get() {
            (session.on_size_known)(None);
        }
        (session.on_done)(VirtualFileResult::Done);
        Ok(())
    }

    fn virtual_file_error(&self, error: VirtualFileError) -> NativeExtensionsResult<()> {
        let session = self
            .virtual_sessions
            .borrow_mut()
            .remove(&error.session_id)
            .ok_or(NativeExtensionsError::VirtualFileSessionNotFound)?;
        if !session.size_known.get() {
            (session.on_size_known)(None);
        }
        (session.on_done)(VirtualFileResult::Error {
            message: error.error_message,
        });
        Ok(())
    }

    fn virtual_file_cancel(&self, complete: VirtualFileCancel) -> NativeExtensionsResult<()> {
        let session = self
            .virtual_sessions
            .borrow_mut()
            .remove(&complete.session_id)
            .ok_or(NativeExtensionsError::VirtualFileSessionNotFound)?;
        if !session.size_known.get() {
            (session.on_size_known)(None);
        }
        (session.on_done)(VirtualFileResult::Cancelled);
        Ok(())
    }
}

#[async_trait(?Send)]
impl PlatformDataProviderDelegate for DataProviderManager {
    fn get_lazy_data(
        &self,
        isolate_id: IsolateId,
        data_id: DataProviderValueId,
        on_done: Option<Box<dyn FnOnce()>>,
    ) -> Arc<ValuePromise> {
        let res = Arc::new(ValuePromise::new());
        let res_clone = res.clone();
        let weak_self = self.weak_self.clone();
        spawn(async move {
            let this = weak_self.upgrade();
            if let Some(this) = this {
                let res = this.get_lazy_data_async(isolate_id, data_id).await;
                res_clone.set(res);
                if let Some(on_done) = on_done {
                    on_done();
                }
            } else {
                res_clone.cancel();
            }
        });
        res
    }

    async fn get_lazy_data_async(
        &self,
        isolate_id: IsolateId,
        value_id: DataProviderValueId,
    ) -> ValuePromiseResult {
        #[derive(IntoValue)]
        #[irondash(rename_all = "camelCase")]
        struct LazyDataRequest {
            value_id: DataProviderValueId,
        }

        let res = self
            .invoker
            .call_method_cv(isolate_id, "getLazyData", LazyDataRequest { value_id })
            .await;
        match res {
            Ok(res) => res,
            Err(_) => ValuePromiseResult::Cancelled,
        }
    }

    fn get_virtual_file(
        &self,
        isolate_id: IsolateId,
        virtual_file_id: DataProviderValueId,
        stream_handle: i32,
        on_size_known: Box<dyn Fn(Option<i64>)>,
        on_progress: Box<dyn Fn(f64 /* 0.0 - 1.0 */)>,
        on_done: Box<dyn FnOnce(VirtualFileResult)>,
    ) -> Arc<VirtualSessionHandle> {
        let weak_self = self.weak_self.clone();
        let session_id: VirtualSessionId = self.next_id.next_id().into();
        let sesion = VirtualFileSession {
            isolate_id,
            size_known: Cell::new(false),
            on_size_known,
            on_progress,
            on_done,
        };
        self.virtual_sessions
            .borrow_mut()
            .insert(session_id, sesion);
        #[derive(IntoValue)]
        #[irondash(rename_all = "camelCase")]
        struct VirtualFileRequest {
            session_id: VirtualSessionId,
            virtual_file_id: DataProviderValueId,
            stream_handle: i32,
        }
        self.invoker.call_method_sync(
            isolate_id,
            "getVirtualFile",
            VirtualFileRequest {
                session_id,
                virtual_file_id,
                stream_handle,
            },
            |r| {
                r.ok_log();
            },
        );
        Arc::new(VirtualSessionHandle(DropNotifier::new(move || {
            if let Some(this) = weak_self.upgrade() {
                this.invoker
                    .call_method_sync(isolate_id, "cancelVirtualFile", session_id, |r| {
                        r.ok_log();
                    });
            }
        })))
    }
}

#[derive(Debug, TryFromValue)]
#[irondash(rename_all = "camelCase")]
struct VirtualFileUpdateProgress {
    session_id: VirtualSessionId,
    progress: f64,
}

#[derive(Debug, TryFromValue)]
#[irondash(rename_all = "camelCase")]
struct VirtualFileSizeKnown {
    session_id: VirtualSessionId,
    file_size: i64,
}

#[derive(Debug, TryFromValue)]
#[irondash(rename_all = "camelCase")]
struct VirtualFileComplete {
    session_id: VirtualSessionId,
}

#[derive(Debug, TryFromValue)]
#[irondash(rename_all = "camelCase")]
struct VirtualFileCancel {
    session_id: VirtualSessionId,
}

#[derive(Debug, TryFromValue)]
#[irondash(rename_all = "camelCase")]
struct VirtualFileError {
    session_id: VirtualSessionId,
    error_message: String,
}

#[async_trait(?Send)]
impl AsyncMethodHandler for DataProviderManager {
    async fn on_method_call(&self, call: MethodCall) -> PlatformResult {
        match call.method.as_str() {
            "registerDataProvider" => self
                .register_provider(call.args.try_into()?, call.isolate)
                .into_platform_result(),
            "unregisterDataProvider" => self
                .unregister_provider(call.args.try_into()?)
                .into_platform_result(),
            "virtualFileUpdateProgress" => self
                .virtual_file_update_progress(call.args.try_into()?)
                .into_platform_result(),
            "virtualFileSizeKnown" => self
                .virtual_file_size_known(call.args.try_into()?)
                .into_platform_result(),
            "virtualFileComplete" => self
                .virtual_file_complete(call.args.try_into()?)
                .into_platform_result(),
            "virtualFileError" => self
                .virtual_file_error(call.args.try_into()?)
                .into_platform_result(),
            "virtualFileCancel" => self
                .virtual_file_cancel(call.args.try_into()?)
                .into_platform_result(),
            _ => Err(PlatformError {
                code: "invalid_method".into(),
                message: Some(format!("Unknown Method: {}", call.method)),
                detail: Value::Null,
            }),
        }
    }

    fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    fn assign_invoker(&self, invoker: AsyncMethodInvoker) {
        self.invoker.set(invoker);
    }

    // Called when engine is about to be destroyed.
    fn on_isolate_destroyed(&self, isolate_id: IsolateId) {
        let mut providers = self.providers.borrow_mut();
        let providers_to_remove: Vec<_> = providers
            .iter()
            .filter_map(|(id, source)| {
                if source.isolate_id == isolate_id {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        for source_id in providers_to_remove {
            providers.remove(&source_id);
        }

        let sessions_to_remove: Vec<_> = {
            self.virtual_sessions
                .borrow()
                .iter()
                .filter_map(|(k, v)| {
                    if v.isolate_id == isolate_id {
                        Some(*k)
                    } else {
                        None
                    }
                })
                .collect()
        };
        for session_id in sessions_to_remove {
            self.virtual_file_cancel(VirtualFileCancel { session_id })
                .ok_log();
        }
    }
}

// FFI

#[no_mangle]
pub extern "C" fn super_native_extensions_stream_write(
    handle: i32,
    data: *mut c_void,
    len: i64,
) -> i32 {
    let buf = unsafe { slice::from_raw_parts(data as *const u8, len as usize) };
    platform_stream_write(handle, buf)
}

#[no_mangle]
pub extern "C" fn super_native_extensions_stream_close(handle: i32, delete: bool) {
    platform_stream_close(handle, delete);
}
