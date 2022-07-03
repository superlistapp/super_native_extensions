use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use async_trait::async_trait;
use nativeshell_core::{
    util::Late, AsyncMethodHandler, AsyncMethodInvoker, Context, IntoPlatformResult, IsolateId,
    PlatformResult, RegisteredAsyncMethodHandler, TryFromValue, Value,
};

use crate::{
    error::{NativeExtensionsError, NativeExtensionsResult},
    platform_impl::platform::PlatformDropContext,
};

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
#[nativeshell(rename_all = "camelCase")]
struct DropContextInitRequest {
    view_handle: i64,
}

#[derive(TryFromValue)]
#[nativeshell(rename_all = "camelCase")]
struct RegisterDropTypesRequest {
    types: Vec<String>,
}

#[async_trait(?Send)]
pub trait PlatformDropContextDelegate {}

impl DropManager {
    pub fn new() -> RegisteredAsyncMethodHandler<Self> {
        Self {
            weak_self: Late::new(),
            invoker: Late::new(),
            contexts: RefCell::new(HashMap::new()),
        }
        .register("DropManager")
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

    fn new_context(
        &self,
        isolate: IsolateId,
        request: DropContextInitRequest,
    ) -> NativeExtensionsResult<()> {
        let context = Rc::new(PlatformDropContext::new(
            isolate,
            request.view_handle,
            self.weak_self.clone(),
        ));
        context.assign_weak_self(Rc::downgrade(&context))?;
        self.contexts.borrow_mut().insert(isolate, context);
        Ok(())
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

    async fn on_method_call(&self, call: nativeshell_core::MethodCall) -> PlatformResult {
        match call.method.as_str() {
            "newContext" => {
                self.new_context(call.isolate, call.args.try_into()?)?;
                Ok(Value::Null)
            }
            "registerDropTypes" => self
                .register_drop_types(call.isolate, call.args.try_into()?)
                .into_platform_result(),
            _ => Ok(Value::Null),
        }
    }

    fn on_isolate_destroyed(&self, isolate: IsolateId) {
        self.contexts.borrow_mut().remove(&isolate);
    }
}

impl PlatformDropContextDelegate for DropManager {}
