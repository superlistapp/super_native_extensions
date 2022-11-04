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
    api_model::{DataProviderId, DragConfiguration, DragItem, DragRequest, DropOperation, Point},
    data_provider_manager::{DataProviderHandle, GetDataProviderManager},
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::{OkLog, OkLogUnexpected},
    platform_impl::platform::{PlatformDataProvider, PlatformDragContext},
    util::{DropNotifier, NextId},
    value_promise::{Promise, PromiseResult},
};

// Each isolate has its own DragContext.
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

pub struct GetAdditionalItemsResult {
    pub items: Vec<DragItem>,
    pub providers: HashMap<DataProviderId, DataProviderEntry>,
}

pub trait PlatformDragContextDelegate {
    fn get_drag_configuration_for_location(
        &self,
        id: PlatformDragContextId,
        location: Point,
    ) -> Arc<Promise<PromiseResult<GetDragConfigurationResult>>>;

    fn get_additional_items_for_location(
        &self,
        id: PlatformDragContextId,
        session_id: DragSessionId,
        location: Point,
    ) -> Arc<Promise<PromiseResult<GetAdditionalItemsResult>>>;

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
    engine_handle: i64,
}

#[derive(TryFromValue)]
#[nativeshell(rename_all = "camelCase")]
pub struct LocalDataRequest {
    session_id: DragSessionId,
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
            request.engine_handle,
            self.weak_self.clone(),
        )?);
        context.assign_weak_self(Rc::downgrade(&context));
        self.contexts.borrow_mut().insert(isolate, context);
        Ok(())
    }

    pub fn get_platform_drag_contexts(&self) -> Vec<Rc<PlatformDragContext>> {
        self.contexts.borrow().values().cloned().collect()
    }

    fn build_data_provider_map(
        &self,
        isolate: IsolateId,
        items: &Vec<DragItem>,
    ) -> NativeExtensionsResult<HashMap<DataProviderId, DataProviderEntry>> {
        let mut map = HashMap::new();
        for item in items {
            let provider_id = item.data_provider_id;
            let provider = Context::get()
                .data_provider_manager()
                .get_platform_data_provider(provider_id)?;
            let weak_self = self.weak_self.clone();
            let handle: DataProviderHandle = DropNotifier::new(move || {
                if let Some(this) = weak_self.upgrade() {
                    // Isolate could have been destroyed in the meanwhile.
                    if this.contexts.borrow().contains_key(&isolate) {
                        this.release_data_provider(isolate, provider_id);
                    }
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
        #[derive(IntoValue)]
        #[nativeshell(rename_all = "camelCase")]
        struct DragConfigurationRequest {
            session_id: DragSessionId,
            location: Point,
        }
        #[derive(TryFromValue, Debug)]
        #[nativeshell(rename_all = "camelCase")]
        struct DragConfigurationResponse {
            configuration: Option<DragConfiguration>,
        }

        let configuration: DragConfigurationResponse = self
            .invoker
            .call_method_cv(
                id,
                "getConfigurationForDragRequest",
                DragConfigurationRequest {
                    location,
                    session_id,
                },
            )
            .await?;
        let configuration = configuration.configuration;
        match configuration {
            Some(configuration) => {
                let providers = self.build_data_provider_map(id, &configuration.items)?;
                Ok(Some(GetDragConfigurationResult {
                    session_id,
                    configuration,
                    providers,
                }))
            }
            None => Ok(None),
        }
    }

    async fn get_additional_items_for_location(
        &self,
        id: PlatformDragContextId,
        session_id: DragSessionId,
        location: Point,
    ) -> NativeExtensionsResult<Option<GetAdditionalItemsResult>> {
        #[derive(IntoValue)]
        #[nativeshell(rename_all = "camelCase")]
        struct AdditionalItemsRequest {
            session_id: DragSessionId,
            location: Point,
        }
        #[derive(TryFromValue, Debug)]
        #[nativeshell(rename_all = "camelCase")]
        struct AdditionalItemsResponse {
            items: Option<Vec<DragItem>>,
        }
        let response: AdditionalItemsResponse = self
            .invoker
            .call_method_cv(
                id,
                "getAdditionalItemsForLocation",
                AdditionalItemsRequest {
                    location,
                    session_id,
                },
            )
            .await?;
        match response.items {
            Some(items) => {
                let providers = self.build_data_provider_map(id, &items)?;
                Ok(Some(GetAdditionalItemsResult { items, providers }))
            }
            None => Ok(None),
        }
    }

    async fn is_location_draggable(
        &self,
        id: PlatformDragContextId,
        location: Point,
    ) -> NativeExtensionsResult<bool> {
        #[derive(IntoValue)]
        #[nativeshell(rename_all = "camelCase")]
        struct LocationDraggableRequest {
            location: Point,
        }
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
            .ok_or(NativeExtensionsError::PlatformContextNotFound)?;
        let session_id = DragSessionId(self.next_session_id.next_id());
        let provider_map = self.build_data_provider_map(isolate, &request.configuration.items)?;
        context
            .start_drag(request, provider_map, session_id)
            .await?;
        Ok(session_id)
    }

    fn get_local_data(
        &self,
        isolate: IsolateId,
        request: LocalDataRequest,
    ) -> NativeExtensionsResult<Option<Vec<Value>>> {
        let context = self
            .contexts
            .borrow()
            .get(&isolate)
            .cloned()
            .ok_or(NativeExtensionsError::PlatformContextNotFound)?;
        match context.get_local_data_for_session_id(request.session_id) {
            Ok(value) => Ok(Some(value)),
            Err(NativeExtensionsError::DragSessionNotFound) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn release_data_provider(&self, isolate_id: IsolateId, provider_id: DataProviderId) {
        self.invoker
            .call_method_sync(isolate_id, "releaseDataProvider", provider_id, |r| {
                r.ok_log();
            })
    }

    fn needs_combined_drag_image(&self) -> NativeExtensionsResult<bool> {
        Ok(PlatformDragContext::needs_combined_drag_image())
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
            "needsCombinedDragImage" => self.needs_combined_drag_image().into_platform_result(),
            "startDrag" => self
                .start_drag(call.isolate, call.args.try_into()?)
                .await
                .into_platform_result(),
            "getLocalData" => self
                .get_local_data(call.isolate, call.args.try_into()?)
                .into_platform_result(),
            _ => Ok(Value::Null),
        }
    }

    fn on_isolate_destroyed(&self, isolate: IsolateId) {
        self.contexts.borrow_mut().remove(&isolate);
    }
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

    fn get_additional_items_for_location(
        &self,
        id: PlatformDragContextId,
        session_id: DragSessionId,
        location: Point,
    ) -> Arc<Promise<PromiseResult<GetAdditionalItemsResult>>> {
        let res = Arc::new(Promise::new());
        let res_clone = res.clone();
        let weak_self = self.weak_self.clone();
        Context::get().run_loop().spawn(async move {
            let this = weak_self.upgrade();
            if let Some(this) = this {
                match this
                    .get_additional_items_for_location(id, session_id, location)
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
        #[derive(IntoValue)]
        #[nativeshell(rename_all = "camelCase")]
        struct DragMoveRequest {
            session_id: DragSessionId,
            screen_location: Point,
        }
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
        #[derive(IntoValue)]
        #[nativeshell(rename_all = "camelCase")]
        struct DragEndRequest {
            session_id: DragSessionId,
            drop_operation: DropOperation,
        }

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
