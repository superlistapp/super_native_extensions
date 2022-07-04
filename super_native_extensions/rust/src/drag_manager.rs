use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
    sync::Arc,
};

use async_trait::async_trait;
use nativeshell_core::{
    util::Late, AsyncMethodHandler, AsyncMethodInvoker, Context, IntoPlatformResult, IntoValue,
    IsolateId, MethodCallError, PlatformResult, RegisteredAsyncMethodHandler, TryFromValue, Value,
};

use crate::{
    api_model::{DataSourceId, ImageData, Point},
    data_source_manager::GetDataSourceManager,
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    platform_impl::platform::{PlatformDataSource, PlatformDragContext},
    util::DropNotifier,
};

pub type PlatformDragContextId = IsolateId;

pub enum PendingSourceState {
    Pending,
    Ok {
        source: Rc<PlatformDataSource>,
        source_drop_notifier: Arc<DropNotifier>,
    },
    Cancelled,
}

pub type WriterResult = Rc<RefCell<PendingSourceState>>;

#[async_trait(?Send)]
pub trait PlatformDragContextDelegate {
    fn data_source_for_drag_request(
        &self,
        id: PlatformDragContextId,
        location: Point,
    ) -> WriterResult;
}

pub struct DragManager {
    weak_self: Late<Weak<Self>>,
    invoker: Late<AsyncMethodInvoker>,
    contexts: RefCell<HashMap<PlatformDragContextId, Rc<PlatformDragContext>>>,
}

pub trait GetDragManager {
    fn drag_manager(&self) -> Rc<DragManager>;
}

impl GetDragManager for Context {
    fn drag_manager(&self) -> Rc<DragManager> {
        self.get_attachment(DragManager::new).handler()
    }
}

#[derive(TryFromValue)]
#[nativeshell(rename_all = "camelCase")]
struct DragContextInitRequest {
    view_handle: i64,
}

#[derive(TryFromValue)]
#[nativeshell(rename_all = "camelCase")]
pub struct DragRequest {
    pub data_source_id: DataSourceId,
    pub point_in_rect: Point,
    pub drag_position: Point,
    pub image: ImageData,
}

impl DragManager {
    pub fn new() -> RegisteredAsyncMethodHandler<Self> {
        Self {
            weak_self: Late::new(),
            invoker: Late::new(),
            contexts: RefCell::new(HashMap::new()),
        }
        .register("DragManager")
    }

    fn new_context(
        &self,
        isolate: IsolateId,
        request: DragContextInitRequest,
    ) -> NativeExtensionsResult<()> {
        let context = Rc::new(PlatformDragContext::new(
            isolate,
            request.view_handle,
            self.weak_self.clone(),
        ));
        context.assign_weak_self(Rc::downgrade(&context));
        self.contexts.borrow_mut().insert(isolate, context);
        Ok(())
    }

    async fn start_drag(
        &self,
        isolate: IsolateId,
        request: DragRequest,
    ) -> NativeExtensionsResult<()> {
        let context = self
            .contexts
            .borrow()
            .get(&isolate)
            .cloned()
            .ok_or_else(|| NativeExtensionsError::PlatformContextNotFound)?;
        let data_source = Context::get()
            .data_source_manager()
            .get_platform_data_source(request.data_source_id)?;

        let weak_self = self.weak_self.clone();
        let source_id = request.data_source_id;
        let notifier = DropNotifier::new(move || {
            if let Some(this) = weak_self.upgrade() {
                this.on_dropped(isolate, source_id);
            }
        });
        context.start_drag(request, data_source, notifier).await
    }

    fn on_dropped(&self, isolate_id: IsolateId, source_id: DataSourceId) {
        self.invoker
            .call_method_sync(isolate_id, "releaseDataSource", source_id, |_| {})
    }
}

#[async_trait(?Send)]
impl AsyncMethodHandler for DragManager {
    fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    fn assign_invoker(&self, invoker: AsyncMethodInvoker) {
        self.invoker.set(invoker);
    }

    async fn on_method_call(&self, call: nativeshell_core::MethodCall) -> PlatformResult {
        match call.method.as_str() {
            "newContext" => {
                self.new_context(call.isolate, call.args.try_into()?)?;
                Ok(Value::Null)
            }
            "startDrag" => self
                .start_drag(call.isolate, call.args.try_into()?)
                .await
                .into_platform_result(),
            _ => Ok(Value::Null),
        }
    }

    fn on_isolate_destroyed(&self, isolate: IsolateId) {
        self.contexts.borrow_mut().remove(&isolate);
    }
}

#[derive(IntoValue)]
#[nativeshell(rename_all = "camelCase")]
struct DataSourceRequest {
    location: Point,
}

#[derive(TryFromValue)]
#[nativeshell(rename_all = "camelCase")]
struct DataSourceResponse {
    data_source_id: Option<DataSourceId>,
}

#[async_trait(?Send)]
impl PlatformDragContextDelegate for DragManager {
    fn data_source_for_drag_request(
        &self,
        id: PlatformDragContextId,
        location: Point,
    ) -> WriterResult {
        let res = Rc::new(RefCell::new(PendingSourceState::Pending));
        let res_clone = res.clone();
        let weak_self = self.weak_self.clone();
        Context::get().run_loop().spawn(async move {
            let this = weak_self.upgrade();
            if let Some(this) = this {
                let data_source: Result<DataSourceResponse, MethodCallError> = this
                    .invoker
                    .call_method_cv(
                        id,
                        "dataSourceForDragRequest",
                        DataSourceRequest { location },
                    )
                    .await;

                let data_source = data_source
                    .ok_log()
                    .and_then(|d| d.data_source_id)
                    .and_then(|d| {
                        Context::get()
                            .data_source_manager()
                            .get_platform_data_source(d)
                            .ok()
                            .map(|s| (d, s))
                    });
                match data_source {
                    Some((data_source_id, data_source)) => {
                        let notifier = DropNotifier::new(move || {
                            if let Some(this) = weak_self.upgrade() {
                                this.on_dropped(id, data_source_id);
                            }
                        });
                        res_clone.replace(PendingSourceState::Ok {
                            source: data_source,
                            source_drop_notifier: notifier,
                        })
                    }
                    None => res_clone.replace(PendingSourceState::Cancelled),
                };
            } else {
                res_clone.replace(PendingSourceState::Cancelled);
            }
        });
        res
    }
}