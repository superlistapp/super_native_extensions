use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use async_trait::async_trait;
use nativeshell_core::{
    util::Late, AsyncMethodHandler, AsyncMethodInvoker, Context, IntoPlatformResult, IntoValue,
    IsolateId, PlatformResult, RegisteredAsyncMethodHandler, TryFromValue, Value,
};

use crate::{
    error::{NativeExtensionsError, NativeExtensionsResult},
    platform_impl::platform::{PlatformDragContext, PlatformDragContextId},
    writer_manager::GetClipboardWriterManager,
};

pub struct DragDropManager {
    weak_self: Late<Weak<Self>>,
    invoker: Late<AsyncMethodInvoker>,
    contexts: RefCell<HashMap<PlatformDragContextId, Rc<PlatformDragContext>>>,
}

#[derive(TryFromValue)]
#[nativeshell(rename_all = "camelCase")]
struct DragDropInitRequest {
    view_handle: i64,
}

#[derive(TryFromValue)]
#[nativeshell(rename_all = "camelCase")]
struct RegisterDropTypesRequest {
    types: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, TryFromValue, IntoValue)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(TryFromValue)]
pub struct DragRequest {
    pub rect: Rect,
    pub writer_id: i64,
}

pub trait GetDragDropManager {
    fn drag_drop_manager(&self) -> Rc<DragDropManager>;
}

impl GetDragDropManager for Context {
    fn drag_drop_manager(&self) -> Rc<DragDropManager> {
        self.get_attachment(DragDropManager::new).handler()
    }
}

impl DragDropManager {
    pub fn new() -> RegisteredAsyncMethodHandler<Self> {
        Self {
            weak_self: Late::new(),
            invoker: Late::new(),
            contexts: RefCell::new(HashMap::new()),
        }
        .register("DragDropManager")
    }

    fn new_context(
        &self,
        isolate: IsolateId,
        request: DragDropInitRequest,
    ) -> NativeExtensionsResult<()> {
        let context = Rc::new(PlatformDragContext::new(isolate, request.view_handle));
        context.assign_weak_self(Rc::downgrade(&context));
        self.contexts.borrow_mut().insert(isolate, context);
        Ok(())
    }

    fn register_drop_types(
        &self,
        isolate: IsolateId,
        request: RegisterDropTypesRequest,
    ) -> NativeExtensionsResult<()> {
        let context = self
            .contexts
            .borrow()
            .get(&isolate)
            .cloned()
            .ok_or_else(|| NativeExtensionsError::PlatformContextNotFound)?;
        context.register_drop_types(&request.types)
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
        let writer = Context::get()
            .clipboard_writer_manager()
            .get_platform_writer(request.writer_id)?;
        context.start_drag(request, writer).await
    }
}

#[async_trait(?Send)]
impl AsyncMethodHandler for DragDropManager {
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
            "registerDropTypes" => self
                .register_drop_types(call.isolate, call.args.try_into()?)
                .into_platform_result(),
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
