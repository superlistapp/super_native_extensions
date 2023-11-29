use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
    sync::Arc,
};

use async_trait::async_trait;
use irondash_message_channel::{
    AsyncMethodHandler, AsyncMethodInvoker, IntoPlatformResult, IntoValue, IsolateId, Late,
    MethodCall, MethodCallError, PlatformResult, RegisteredAsyncMethodHandler, TryFromValue, Value,
};
use irondash_run_loop::{spawn, RunLoop};
use log::warn;

use crate::{
    api_model::{DropOperation, ImageData, Point, Rect, Size},
    context::Context,
    drag_manager::{GetDragManager, PlatformDragContextId},
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::{OkLog, OkLogUnexpected},
    platform_impl::platform::{PlatformDataReader, PlatformDragContext, PlatformDropContext},
    reader_manager::{GetDataReaderManager, RegisteredDataReader},
    value_promise::{Promise, PromiseResult},
};

// Each isolate has its own DropContext.
pub type PlatformDropContextId = IsolateId;

pub struct DropManager {
    weak_self: Late<Weak<Self>>,
    invoker: Late<AsyncMethodInvoker>,
    contexts: RefCell<HashMap<PlatformDropContextId, Rc<PlatformDropContext>>>,
}

pub trait GetDropManager {
    fn drop_manager(&self) -> Rc<DropManager>;
}

impl GetDropManager for Context {
    fn drop_manager(&self) -> Rc<DropManager> {
        self.get_attachment(DropManager::new).handler()
    }
}

#[derive(TryFromValue)]
#[irondash(rename_all = "camelCase")]
struct DropContextInitRequest {
    engine_handle: i64,
}

#[derive(TryFromValue)]
#[irondash(rename_all = "camelCase")]
struct RegisterDropFormatsRequest {
    formats: Vec<String>,
}

#[derive(Debug, TryFromValue, IntoValue, Clone, Copy, PartialEq, Hash, Eq)]
pub struct DropSessionId(i64);

impl From<isize> for DropSessionId {
    fn from(v: isize) -> Self {
        Self(v as i64)
    }
}

impl From<i64> for DropSessionId {
    fn from(v: i64) -> Self {
        Self(v)
    }
}

#[derive(Debug, TryFromValue, IntoValue, Clone, Copy, PartialEq, Hash, Eq)]
pub struct DropItemId(i64);

impl From<i64> for DropItemId {
    fn from(v: i64) -> Self {
        Self(v)
    }
}

#[derive(IntoValue, Debug)]
#[irondash(rename_all = "camelCase")]
pub struct DropItem {
    pub item_id: DropItemId, // unique ID within session, consistent between events
    pub formats: Vec<String>,
    pub local_data: Value,
}

#[derive(IntoValue, Debug)]
#[irondash(rename_all = "camelCase")]
pub struct DropEvent {
    pub session_id: DropSessionId,
    pub location_in_view: Point,
    pub allowed_operations: Vec<DropOperation>,
    pub accepted_operation: Option<DropOperation>,
    pub items: Vec<DropItem>,
    pub reader: Option<RegisteredDataReader>,
}

#[derive(IntoValue, Debug)]
#[irondash(rename_all = "camelCase")]
pub struct BaseDropEvent {
    pub session_id: DropSessionId,
}

#[derive(IntoValue)]
#[irondash(rename_all = "camelCase")]
pub struct ItemPreviewRequest {
    pub session_id: DropSessionId,
    pub item_id: DropItemId,
    pub size: Size,
    pub fade_out_delay: f64,    // delay before preview starts fading out
    pub fade_out_duration: f64, // duration of fade out animation
}

#[derive(TryFromValue)]
#[irondash(rename_all = "camelCase")]
pub struct ItemPreview {
    pub destination_image: Option<ImageData>,
    pub destination_rect: Rect,
    pub fade_out_delay: Option<f64>,
    pub fade_out_duration: Option<f64>,
}

#[derive(TryFromValue)]
#[irondash(rename_all = "camelCase")]
pub struct ItemPreviewResponse {
    pub preview: Option<ItemPreview>,
}

pub trait PlatformDropContextDelegate {
    fn get_platform_drag_contexts(&self) -> Vec<Rc<PlatformDragContext>>;

    fn send_drop_update(
        &self,
        id: PlatformDropContextId,
        event: DropEvent,
        res: Box<dyn FnOnce(Result<DropOperation, MethodCallError>)>,
    );

    fn send_perform_drop(
        &self,
        id: PlatformDropContextId,
        event: DropEvent,
        res: Box<dyn FnOnce(Result<(), MethodCallError>)>,
    );

    fn send_drop_leave(&self, id: PlatformDropContextId, event: BaseDropEvent);

    fn send_drop_ended(&self, id: PlatformDropContextId, event: BaseDropEvent);

    fn register_platform_reader(
        &self,
        id: PlatformDropContextId,
        platform_reader: Rc<PlatformDataReader>,
    ) -> RegisteredDataReader;

    fn get_preview_for_item(
        &self,
        id: PlatformDropContextId,
        request: ItemPreviewRequest,
    ) -> Arc<Promise<PromiseResult<ItemPreviewResponse>>>;
}

impl DropManager {
    pub fn new() -> RegisteredAsyncMethodHandler<Self> {
        Self {
            weak_self: Late::new(),
            invoker: Late::new(),
            contexts: RefCell::new(HashMap::new()),
        }
        .register("DropManager")
    }

    fn register_drop_formats(
        &self,
        isolate: IsolateId,
        request: RegisterDropFormatsRequest,
    ) -> NativeExtensionsResult<()> {
        let context = self
            .contexts
            .borrow()
            .get(&isolate)
            .cloned()
            .ok_or(NativeExtensionsError::PlatformContextNotFound)?;
        context.register_drop_formats(&request.formats)
    }

    fn new_context(
        &self,
        isolate: IsolateId,
        request: DropContextInitRequest,
    ) -> NativeExtensionsResult<()> {
        if self.contexts.borrow().get(&isolate).is_some() {
            // Can happen during hot reload
            warn!("DropContext already exists for isolate {:?}", isolate);
            return Ok(());
        }
        let context = Rc::new(PlatformDropContext::new(
            isolate,
            request.engine_handle,
            self.weak_self.clone(),
        )?);
        context.assign_weak_self(Rc::downgrade(&context));
        self.contexts.borrow_mut().insert(isolate, context);
        Ok(())
    }

    pub fn get_platform_drop_contexts(&self) -> Vec<Rc<PlatformDropContext>> {
        self.contexts.borrow().values().cloned().collect()
    }

    async fn get_preview_for_item(
        &self,
        id: PlatformDropContextId,
        request: ItemPreviewRequest,
    ) -> NativeExtensionsResult<ItemPreviewResponse> {
        let result = self
            .invoker
            .call_method_cv(id, "getPreviewForItem", request)
            .await?;
        Ok(result)
    }
}

#[async_trait(?Send)]
impl AsyncMethodHandler for DropManager {
    fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    fn assign_invoker(&self, invoker: AsyncMethodInvoker) {
        self.invoker.set(invoker);
    }

    async fn on_method_call(&self, call: MethodCall) -> PlatformResult {
        match call.method.as_str() {
            "newContext" => {
                self.new_context(call.isolate, call.args.try_into()?)?;
                Ok(Value::Null)
            }
            "registerDropFormats" => self
                .register_drop_formats(call.isolate, call.args.try_into()?)
                .into_platform_result(),
            _ => Ok(Value::Null),
        }
    }

    fn on_isolate_destroyed(&self, isolate: IsolateId) {
        self.contexts.borrow_mut().remove(&isolate);
    }
}

impl PlatformDropContextDelegate for DropManager {
    fn get_platform_drag_contexts(&self) -> Vec<Rc<PlatformDragContext>> {
        Context::get().drag_manager().get_platform_drag_contexts()
    }

    fn send_drop_update(
        &self,
        id: PlatformDropContextId,
        event: DropEvent,
        res: Box<dyn FnOnce(Result<DropOperation, MethodCallError>)>,
    ) {
        self.invoker
            .call_method_sync_cv(id, "onDropUpdate", event, res);
    }

    fn send_perform_drop(
        &self,
        id: PlatformDropContextId,
        event: DropEvent,
        res: Box<dyn FnOnce(Result<(), MethodCallError>)>,
    ) {
        self.invoker
            .call_method_sync_cv(id, "onPerformDrop", event, |r| {
                // Delay result callback one run loop turn. This is necessary because
                // AsyncMethodHandler::on_message executes messages using RunLoop::spawn,
                // whcih means that calls such as PlatformReader::get_data_for_item are delayed
                // one run loop turn. It is necessary for the result callback not to be invoked
                // before dispatching any calls received during perform_drop.
                // Not doing so would result in race condition on iOS where drop data
                // must only be received during perform_drop.
                RunLoop::current().schedule_next(move || res(r)).detach();
            });
    }

    fn send_drop_leave(&self, id: PlatformDropContextId, event: BaseDropEvent) {
        self.invoker
            .call_method_sync(id, "onDropLeave", event, |r| {
                r.ok_log();
            });
    }

    fn send_drop_ended(&self, id: PlatformDropContextId, event: BaseDropEvent) {
        self.invoker
            .call_method_sync(id, "onDropEnded", event, |r| {
                r.ok_log();
            });
    }

    fn register_platform_reader(
        &self,
        id: PlatformDropContextId,
        platform_reader: Rc<PlatformDataReader>,
    ) -> RegisteredDataReader {
        Context::get()
            .data_reader_manager()
            .register_platform_reader(platform_reader, id)
    }

    fn get_preview_for_item(
        &self,
        id: PlatformDragContextId,
        request: ItemPreviewRequest,
    ) -> Arc<Promise<PromiseResult<ItemPreviewResponse>>> {
        let res = Arc::new(Promise::new());
        let res_clone = res.clone();
        let weak_self = self.weak_self.clone();
        spawn(async move {
            let this = weak_self.upgrade();
            if let Some(this) = this {
                let draggable = this
                    .get_preview_for_item(id, request)
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
}
