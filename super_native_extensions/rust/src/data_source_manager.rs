use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    sync::Arc,
};

use async_trait::async_trait;
use nativeshell_core::{
    util::Late, AsyncMethodHandler, AsyncMethodInvoker, Context, IntoPlatformResult, IntoValue,
    IsolateId, MethodCall, PlatformError, PlatformResult, RegisteredAsyncMethodHandler, Value,
};

use crate::{
    api_model::{DataSource, DataSourceId, LazyValueId},
    error::{NativeExtensionsError, NativeExtensionsResult},
    platform_impl::platform::PlatformDataSource,
    value_promise::{ValuePromise, ValuePromiseResult},
};

#[async_trait(?Send)]
pub trait PlatformDataSourceDelegate {
    fn get_lazy_data(
        &self,
        isolate_id: IsolateId,
        data_id: LazyValueId,
        format: String,
        on_done: Option<Box<dyn FnOnce()>>,
    ) -> Arc<ValuePromise>;

    async fn get_lazy_data_async(
        &self,
        isolate_id: IsolateId,
        data_id: LazyValueId,
        format: String,
    ) -> ValuePromiseResult;
}

pub struct DataSourceManager {
    weak_self: Late<Weak<Self>>,
    invoker: Late<AsyncMethodInvoker>,
    next_id: Cell<i64>,
    sources: RefCell<HashMap<DataSourceId, DataSourceEntry>>,
}

struct DataSourceEntry {
    isolate_id: IsolateId,
    platform_data_source: Rc<PlatformDataSource>,
}

pub trait GetDataSourceManager {
    fn data_source_manager(&self) -> Rc<DataSourceManager>;
}

impl GetDataSourceManager for Context {
    fn data_source_manager(&self) -> Rc<DataSourceManager> {
        self.get_attachment(DataSourceManager::new).handler()
    }
}

impl DataSourceManager {
    pub fn new() -> RegisteredAsyncMethodHandler<Self> {
        Self {
            weak_self: Late::new(),
            invoker: Late::new(),
            next_id: Cell::new(1),
            sources: RefCell::new(HashMap::new()),
        }
        .register("DataSourceManager")
    }

    pub fn get_platform_data_source(
        &self,
        source_id: DataSourceId,
    ) -> NativeExtensionsResult<Rc<PlatformDataSource>> {
        self.sources
            .borrow()
            .get(&source_id)
            .map(|e| e.platform_data_source.clone())
            .ok_or_else(|| NativeExtensionsError::DataSourceNotFound)
    }

    fn register_source(
        &self,
        source: DataSource,
        isolate_id: IsolateId,
    ) -> NativeExtensionsResult<i64> {
        let id = self.next_id.get();
        self.next_id.replace(id + 1);
        let platform_data_source = Rc::new(PlatformDataSource::new(
            self.weak_self.clone(),
            isolate_id,
            source,
        ));
        platform_data_source.assign_weak_self(Rc::downgrade(&platform_data_source));
        self.sources.borrow_mut().insert(
            id.into(),
            DataSourceEntry {
                isolate_id,
                platform_data_source,
            },
        );
        Ok(id)
    }

    fn unregister_source(&self, source: DataSourceId) -> NativeExtensionsResult<()> {
        self.sources.borrow_mut().remove(&source);
        Ok(())
    }
}

#[async_trait(?Send)]
impl AsyncMethodHandler for DataSourceManager {
    async fn on_method_call(&self, call: MethodCall) -> PlatformResult {
        match call.method.as_str() {
            "registerDataSource" => self
                .register_source(call.args.try_into()?, call.isolate)
                .into_platform_result(),
            "unregisterDataSource" => self
                .unregister_source(call.args.try_into()?)
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
        let mut writers = self.sources.borrow_mut();
        let sources_to_remove: Vec<_> = writers
            .iter()
            .filter_map(|(id, writer)| {
                if writer.isolate_id == isolate_id {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        for writer_id in sources_to_remove {
            writers.remove(&writer_id);
        }
    }
}

#[async_trait(?Send)]
impl PlatformDataSourceDelegate for DataSourceManager {
    fn get_lazy_data(
        &self,
        isolate_id: IsolateId,
        data_id: LazyValueId,
        format: String,
        on_done: Option<Box<dyn FnOnce()>>,
    ) -> Arc<ValuePromise> {
        let res = Arc::new(ValuePromise::new());
        let res_clone = res.clone();
        let weak_self = self.weak_self.clone();
        Context::get().run_loop().spawn(async move {
            let this = weak_self.upgrade();
            if let Some(this) = this {
                let res = this.get_lazy_data_async(isolate_id, data_id, format).await;
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
        data_id: LazyValueId,
        format: String,
    ) -> ValuePromiseResult {
        #[derive(IntoValue)]
        #[nativeshell(rename_all = "camelCase")]
        struct LazyDataRequest {
            id: LazyValueId,
            format: String,
        }

        let res = self
            .invoker
            .call_method_cv(
                isolate_id,
                "getLazyData",
                LazyDataRequest {
                    id: data_id,
                    format,
                },
            )
            .await;
        match res {
            Ok(res) => res,
            Err(_) => ValuePromiseResult::Cancelled,
        }
    }
}
