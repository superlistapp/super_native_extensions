use std::{
    cell::{Cell, RefCell},
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
    api_model::{DataSourceId, DragData, DragRequest, DropOperation, Point},
    data_source_manager::GetDataSourceManager,
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    platform_impl::platform::{PlatformDataSource, PlatformDragContext},
    util::{DropNotifier, NextId},
};

pub type PlatformDragContextId = IsolateId;

pub enum PendingSourceState {
    Pending,
    Ok {
        source: Rc<PlatformDataSource>,
        source_drop_notifier: Arc<DropNotifier>,
        session_id: DragSessionId,
        drag_data: DragData,
    },
    Cancelled,
}

pub type GetDataResult = Rc<RefCell<PendingSourceState>>;

#[async_trait(?Send)]
pub trait PlatformDragContextDelegate {
    fn get_data_for_drag_request(
        &self,
        id: PlatformDragContextId,
        location: Point,
    ) -> GetDataResult;

    fn drag_session_did_end_with_operation(
        &self,
        id: PlatformDragContextId,
        session_id: DragSessionId,
        operation: DropOperation,
    );
}

#[derive(Debug, TryFromValue, IntoValue, Clone, Copy, PartialEq, Hash, Eq)]
pub struct DragSessionId(i64);

pub struct DragManager {
    weak_self: Late<Weak<Self>>,
    invoker: Late<AsyncMethodInvoker>,
    contexts: RefCell<HashMap<PlatformDragContextId, Rc<PlatformDragContext>>>,
    next_session_id: Cell<i64>,
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

impl DragManager {
    pub fn new() -> RegisteredAsyncMethodHandler<Self> {
        Self {
            weak_self: Late::new(),
            invoker: Late::new(),
            contexts: RefCell::new(HashMap::new()),
            next_session_id: Cell::new(0),
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
    ) -> NativeExtensionsResult<DragSessionId> {
        let context = self
            .contexts
            .borrow()
            .get(&isolate)
            .cloned()
            .ok_or_else(|| NativeExtensionsError::PlatformContextNotFound)?;
        let data_source = Context::get()
            .data_source_manager()
            .get_platform_data_source(request.drag_data.data_source_id)?;

        let weak_self = self.weak_self.clone();
        let source_id = request.drag_data.data_source_id;
        let notifier = DropNotifier::new(move || {
            if let Some(this) = weak_self.upgrade() {
                this.release_data_source(isolate, source_id);
            }
        });
        let session_id = DragSessionId(self.next_session_id.next_id());
        context
            .start_drag(request, data_source, notifier, session_id)
            .await?;
        Ok(session_id)
    }

    fn release_data_source(&self, isolate_id: IsolateId, source_id: DataSourceId) {
        self.invoker
            .call_method_sync(isolate_id, "releaseDataSource", source_id, |r| {
                r.ok_log();
            })
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
    session_id: DragSessionId,
}

#[derive(TryFromValue, Debug)]
#[nativeshell(rename_all = "camelCase")]
struct DataResponse {
    drag_data: Option<DragData>,
}

#[derive(IntoValue)]
#[nativeshell(rename_all = "camelCase")]
struct DragEndResponse {
    session_id: DragSessionId,
    drop_operation: DropOperation,
}

#[async_trait(?Send)]
impl PlatformDragContextDelegate for DragManager {
    fn get_data_for_drag_request(
        &self,
        id: PlatformDragContextId,
        location: Point,
    ) -> GetDataResult {
        let res = Rc::new(RefCell::new(PendingSourceState::Pending));
        let res_clone = res.clone();
        let weak_self = self.weak_self.clone();
        let session_id = DragSessionId(self.next_session_id.next_id());
        Context::get().run_loop().spawn(async move {
            let this = weak_self.upgrade();
            if let Some(this) = this {
                let data_source: Result<DataResponse, MethodCallError> = this
                    .invoker
                    .call_method_cv(
                        id,
                        "getDataForDragRequest",
                        DataSourceRequest {
                            location,
                            session_id,
                        },
                    )
                    .await;
                let data_source = data_source
                    .ok_log()
                    .and_then(|d| d.drag_data)
                    .and_then(|d| {
                        Context::get()
                            .data_source_manager()
                            .get_platform_data_source(d.data_source_id)
                            .ok()
                            .map(|s| (d, s))
                    });
                match data_source {
                    Some((drag_data, data_source)) => {
                        let notifier = DropNotifier::new(move || {
                            if let Some(this) = weak_self.upgrade() {
                                this.release_data_source(id, drag_data.data_source_id);
                            }
                        });
                        res_clone.replace(PendingSourceState::Ok {
                            source: data_source,
                            source_drop_notifier: notifier,
                            session_id,
                            drag_data,
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

    fn drag_session_did_end_with_operation(
        &self,
        id: PlatformDragContextId,
        session_id: DragSessionId,
        operation: DropOperation,
    ) {
        self.invoker.call_method_sync(
            id,
            "dragSessionDidEnd",
            DragEndResponse {
                session_id,
                drop_operation: operation,
            },
            |r| {
                r.ok_log();
            },
        );
    }
}
