use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    sync::Arc,
};

use async_trait::async_trait;

use nativeshell_core::{
    util::Late, AsyncMethodHandler, AsyncMethodInvoker, Context, IntoPlatformResult, IntoValue,
    IsolateId, PlatformResult, RegisteredAsyncMethodHandler, TryFromValue, Value,
};

use crate::{
    api_model::{DataProviderId, DragConfiguration, DragRequest, DropOperation, Point},
    data_provider_manager::{DataProviderHandle, GetDataProviderManager},
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::{OkLog, OkLogUnexpected},
    platform_impl::platform::{PlatformDataProvider, PlatformDragContext},
    util::{DropNotifier, NextId},
    value_promise::{Promise, PromiseResult},
};

pub type PlatformDragContextId = IsolateId;

pub struct DataProviderEntry {
    pub provider: Rc<PlatformDataProvider>,
    pub handle: Arc<DataProviderHandle>,
}

pub struct GetDragConfigurationResult {
    pub session_id: DragSessionId,
    pub configuration: DragConfiguration,
    pub providers: HashMap<DataProviderId, DataProviderEntry>,
}

pub trait PlatformDragContextDelegate {
    fn get_drag_configuration_for_location(
        &self,
        id: PlatformDragContextId,
        location: Point,
    ) -> Arc<Promise<PromiseResult<GetDragConfigurationResult>>>;

    fn is_location_draggable(
        &self,
        id: PlatformDragContextId,
        location: Point,
    ) -> Arc<Promise<PromiseResult<bool>>>;

    fn drag_session_did_move_to_location(
        &self,
        id: PlatformDragContextId,
        session_id: DragSessionId,
        screen_location: Point,
    );

    fn drag_session_did_end_with_operation(
        &self,
        id: PlatformDragContextId,
        session_id: DragSessionId,
        operation: DropOperation,
    );
}

#[derive(Debug, TryFromValue, IntoValue, Clone, Copy, PartialEq, Hash, Eq)]
pub struct DragSessionId(i64);

impl From<i64> for DragSessionId {
    fn from(v: i64) -> Self {
        Self(v)
    }
}

impl From<DragSessionId> for i64 {
    fn from(s: DragSessionId) -> Self {
        s.0
    }
}

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

    pub fn get_platform_drag_context(
        &self,
        id: PlatformDragContextId,
    ) -> NativeExtensionsResult<Rc<PlatformDragContext>> {
        self.contexts
            .borrow()
            .get(&id)
            .cloned()
            .ok_or_else(|| NativeExtensionsError::PlatformContextNotFound)
    }

    fn build_data_provider_map(
        &self,
        isolate: IsolateId,
        configuration: &DragConfiguration,
    ) -> NativeExtensionsResult<HashMap<DataProviderId, DataProviderEntry>> {
        let mut map = HashMap::new();
        for item in &configuration.items {
            let provider_id = item.data_provider_id;
            let provider = Context::get()
                .data_provider_manager()
                .get_platform_data_provider(provider_id)?;
            let weak_self = self.weak_self.clone();
            let handle: DataProviderHandle = DropNotifier::new(move || {
                if let Some(this) = weak_self.upgrade() {
                    this.release_data_provider(isolate, provider_id);
                }
            })
            .into();
            map.insert(
                provider_id,
                DataProviderEntry {
                    provider,
                    handle: Arc::new(handle),
                },
            );
        }
        Ok(map)
    }

    async fn get_drag_configuration_for_location(
        &self,
        id: PlatformDragContextId,
        session_id: DragSessionId,
        location: Point,
    ) -> NativeExtensionsResult<Option<GetDragConfigurationResult>> {
        let configuration: DragConfigurationResponse = self
            .invoker
            .call_method_cv(
                id,
                "getConfigurationForDragRequest",
                DataSourceRequest {
                    location,
                    session_id,
                },
            )
            .await?;
        let configuration = configuration.configuration;
        match configuration {
            Some(configuration) => {
                let providers = self.build_data_provider_map(id, &configuration)?;
                Ok(Some(GetDragConfigurationResult {
                    session_id,
                    configuration,
                    providers,
                }))
            }
            None => Ok(None),
        }
    }

    async fn is_location_draggable(
        &self,
        id: PlatformDragContextId,
        location: Point,
    ) -> NativeExtensionsResult<bool> {
        let result: bool = self
            .invoker
            .call_method_cv(
                id,
                "isLocationDraggable",
                LocationDraggableRequest { location },
            )
            .await?;
        Ok(result)
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
        let session_id = DragSessionId(self.next_session_id.next_id());
        let provider_map = self.build_data_provider_map(isolate, &&request.configuration)?;
        context
            .start_drag(request, provider_map, session_id)
            .await?;
        Ok(session_id)
    }

    fn release_data_provider(&self, isolate_id: IsolateId, provider_id: DataProviderId) {
        self.invoker
            .call_method_sync(isolate_id, "releaseDataProvider", provider_id, |r| {
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

#[derive(IntoValue)]
#[nativeshell(rename_all = "camelCase")]
struct LocationDraggableRequest {
    location: Point,
}

#[derive(TryFromValue, Debug)]
#[nativeshell(rename_all = "camelCase")]
struct DragConfigurationResponse {
    configuration: Option<DragConfiguration>,
}

#[derive(IntoValue)]
#[nativeshell(rename_all = "camelCase")]
struct DragMoveRequest {
    session_id: DragSessionId,
    screen_location: Point,
}

#[derive(IntoValue)]
#[nativeshell(rename_all = "camelCase")]
struct DragEndRequest {
    session_id: DragSessionId,
    drop_operation: DropOperation,
}

impl PlatformDragContextDelegate for DragManager {
    fn get_drag_configuration_for_location(
        &self,
        id: PlatformDragContextId,
        location: Point,
    ) -> Arc<Promise<PromiseResult<GetDragConfigurationResult>>> {
        let res = Arc::new(Promise::new());
        let res_clone = res.clone();
        let weak_self = self.weak_self.clone();
        let session_id = DragSessionId(self.next_session_id.next_id());
        Context::get().run_loop().spawn(async move {
            let this = weak_self.upgrade();
            if let Some(this) = this {
                match this
                    .get_drag_configuration_for_location(id, session_id, location)
                    .await
                    .ok_log_unexpected()
                    .flatten()
                {
                    Some(data) => {
                        res_clone.set(PromiseResult::Ok { value: data });
                    }
                    None => {
                        res_clone.set(PromiseResult::Cancelled);
                    }
                }
            } else {
                res_clone.set(PromiseResult::Cancelled);
            }
        });
        res
    }

    fn is_location_draggable(
        &self,
        id: PlatformDragContextId,
        location: Point,
    ) -> Arc<Promise<PromiseResult<bool>>> {
        let res = Arc::new(Promise::new());
        let res_clone = res.clone();
        let weak_self = self.weak_self.clone();
        Context::get().run_loop().spawn(async move {
            let this = weak_self.upgrade();
            if let Some(this) = this {
                let draggable = this
                    .is_location_draggable(id, location)
                    .await
                    .ok_log_unexpected();
                match draggable {
                    Some(draggable) => res_clone.set(PromiseResult::Ok { value: draggable }),
                    None => res_clone.set(PromiseResult::Cancelled),
                }
            } else {
                res_clone.set(PromiseResult::Cancelled);
            }
        });
        res
    }

    fn drag_session_did_move_to_location(
        &self,
        id: PlatformDragContextId,
        session_id: DragSessionId,
        screen_location: Point,
    ) {
        self.invoker.call_method_sync(
            id,
            "dragSessionDidMove",
            DragMoveRequest {
                session_id,
                screen_location,
            },
            |r| {
                r.ok_log();
            },
        )
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
            DragEndRequest {
                session_id,
                drop_operation: operation,
            },
            |r| {
                r.ok_log();
            },
        );
    }
}
