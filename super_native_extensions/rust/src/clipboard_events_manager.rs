use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use async_trait::async_trait;
use irondash_message_channel::{
    AsyncMethodHandler, AsyncMethodInvoker, IsolateId, Late, MethodCall, PlatformError,
    PlatformResult, RegisteredAsyncMethodHandler, Value,
};
use log::warn;

use crate::{
    context::Context, error::NativeExtensionsResult, log::OkLog,
    platform::PlatformClipboardEventManager,
};

// Each isolate has its own DragContext.
pub type PlatformClipboardEventManagerId = IsolateId;

pub struct ClipboardEventManager {
    invoker: Late<AsyncMethodInvoker>,
    platform_managers:
        RefCell<HashMap<PlatformClipboardEventManagerId, Rc<PlatformClipboardEventManager>>>,
    weak_self: Late<Weak<Self>>,
}

#[async_trait(?Send)]
pub trait ClipboardEventManagerDelegate {
    async fn on_cut(&self, isolate_id: IsolateId) -> bool;
    async fn on_copy(&self, isolate_id: IsolateId) -> bool;
    async fn on_paste(&self, isolate_id: IsolateId) -> bool;
    async fn on_select_all(&self, isolate_id: IsolateId) -> bool;
}

pub trait GetClipboardEventManager {
    fn clipboard_event_manager(&self) -> Rc<ClipboardEventManager>;
}

impl GetClipboardEventManager for Context {
    fn clipboard_event_manager(&self) -> Rc<ClipboardEventManager> {
        self.get_attachment(ClipboardEventManager::new).handler()
    }
}

impl ClipboardEventManager {
    pub fn new() -> RegisteredAsyncMethodHandler<Self> {
        Self {
            invoker: Late::new(),
            platform_managers: RefCell::new(HashMap::new()),
            weak_self: Late::new(),
        }
        .register("ClipboardEventManager")
    }

    fn new_clipboard_events_manager(&self, isolate: IsolateId) -> NativeExtensionsResult<()> {
        if self.platform_managers.borrow().get(&isolate).is_some() {
            // Can happen during hot reload
            warn!("DragContext already exists for isolate {:?}", isolate);
            return Ok(());
        }
        let context = Rc::new(PlatformClipboardEventManager::new(
            isolate,
            self.weak_self.clone(),
        ));
        context.assign_weak_self(Rc::downgrade(&context));
        self.platform_managers.borrow_mut().insert(isolate, context);
        Ok(())
    }
}

#[async_trait(?Send)]
impl ClipboardEventManagerDelegate for ClipboardEventManager {
    async fn on_cut(&self, isolate_id: IsolateId) -> bool {
        let res = self
            .invoker
            .call_method_cv(isolate_id, "cut", Value::Null)
            .await;
        res.ok_log().unwrap_or(false)
    }

    async fn on_copy(&self, isolate_id: IsolateId) -> bool {
        let res = self
            .invoker
            .call_method_cv(isolate_id, "copy", Value::Null)
            .await;
        res.ok_log().unwrap_or(false)
    }

    async fn on_paste(&self, isolate_id: IsolateId) -> bool {
        let res = self
            .invoker
            .call_method_cv(isolate_id, "paste", Value::Null)
            .await;
        res.ok_log().unwrap_or(false)
    }

    async fn on_select_all(&self, isolate_id: IsolateId) -> bool {
        let res = self
            .invoker
            .call_method_cv(isolate_id, "selectAll", Value::Null)
            .await;
        res.ok_log().unwrap_or(false)
    }
}

#[async_trait(?Send)]
impl AsyncMethodHandler for ClipboardEventManager {
    fn assign_invoker(&self, invoker: AsyncMethodInvoker) {
        self.invoker.set(invoker);
    }

    fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    async fn on_method_call(&self, call: MethodCall) -> PlatformResult {
        #[allow(clippy::match_single_binding)]
        match call.method.as_str() {
            "newClipboardEventsManager" => {
                self.new_clipboard_events_manager(call.isolate)?;
                Ok(Value::Null)
            }
            _ => Err(PlatformError {
                code: "invalid_method".into(),
                message: Some(format!("Unknown Method: {}", call.method)),
                detail: Value::Null,
            }),
        }
    }
}
