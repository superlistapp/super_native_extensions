use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    convert::TryInto,
    rc::Rc,
};

use irondash_message_channel::{
    IntoPlatformResult, IntoValue, IsolateId, Late, MethodCall, MethodCallReply, MethodHandler,
    MethodInvoker, PlatformResult, RegisteredMethodHandler, TryFromValue, Value,
};

use crate::{
    context::Context,
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    platform_impl::platform::PlatformHotKeyManager,
    util::NextId,
};

#[derive(TryFromValue, Debug, Clone)]
#[irondash(rename_all = "camelCase")]
pub struct HotKeyCreateRequest {
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub control: bool,
    pub platform_code: i64,
}

#[derive(TryFromValue, Debug)]
struct HotKeyDestroyRequest {
    pub handle: HotKeyHandle,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, IntoValue, TryFromValue)]
pub struct HotKeyHandle(i64);

pub struct HotKeyManager {
    invoker: Late<MethodInvoker>,
    handle_to_isolate: RefCell<HashMap<HotKeyHandle, IsolateId>>,
    next_id: Cell<i64>,
    platform_manager: Late<Rc<PlatformHotKeyManager>>,
}

pub trait HotKeyManagerDelegate {
    fn on_hot_key_pressed(&self, handle: HotKeyHandle);
    fn on_hot_key_released(&self, handle: HotKeyHandle);
}

pub trait GetHotKeyManager {
    fn hot_key_manager(&self) -> Rc<HotKeyManager>;
}

impl GetHotKeyManager for Context {
    fn hot_key_manager(&self) -> Rc<HotKeyManager> {
        self.get_attachment(HotKeyManager::new).handler()
    }
}

impl HotKeyManager {
    pub fn new() -> RegisteredMethodHandler<Self> {
        Self {
            invoker: Late::new(),
            handle_to_isolate: RefCell::new(HashMap::new()),
            next_id: Cell::new(1),
            platform_manager: Late::new(),
        }
        .register("HotKeyManager")
    }

    fn create_hot_key(
        &self,
        isolate_id: IsolateId,
        request: HotKeyCreateRequest,
    ) -> NativeExtensionsResult<Option<HotKeyHandle>> {
        let handle = HotKeyHandle(self.next_id.next_id());
        let res = self.platform_manager.create_hot_key(handle, request);
        if let Err(NativeExtensionsError::UnsupportedOperation) = res {
            return Ok(None);
        }
        res?;
        self.handle_to_isolate
            .borrow_mut()
            .insert(handle, isolate_id);
        Ok(Some(handle))
    }

    fn destroy_hot_key(&self, request: HotKeyDestroyRequest) -> NativeExtensionsResult<()> {
        self.handle_to_isolate.borrow_mut().remove(&request.handle);
        self.platform_manager.destroy_hot_key(request.handle)
    }

    fn on_method_call(&self, call: MethodCall) -> PlatformResult {
        match call.method.as_str() {
            "createHotKey" => self
                .create_hot_key(call.isolate, call.args.try_into()?)
                .into_platform_result(),
            "destroyHotKey" => self
                .destroy_hot_key(call.args.try_into()?)
                .into_platform_result(),
            _ => Ok(Value::Null),
        }
    }
}

impl MethodHandler for HotKeyManager {
    fn on_method_call(&self, call: MethodCall, reply: MethodCallReply) {
        reply.send(self.on_method_call(call))
    }

    fn assign_invoker(&self, invoker: MethodInvoker) {
        self.invoker.set(invoker);
    }

    fn assign_weak_self(&self, weak_self: std::rc::Weak<Self>) {
        let platform_manager = Rc::new(PlatformHotKeyManager::new(weak_self));
        platform_manager.assign_weak_self(Rc::downgrade(&platform_manager));
        self.platform_manager.set(platform_manager);
    }

    fn on_isolate_destroyed(&self, isolate: IsolateId) {
        let handles = self
            .handle_to_isolate
            .borrow()
            .iter()
            .filter_map(|(handle, id)| if *id == isolate { Some(*handle) } else { None })
            .collect::<Vec<_>>();
        for handle in handles {
            self.handle_to_isolate.borrow_mut().remove(&handle);
            self.platform_manager.destroy_hot_key(handle).ok_log();
        }
    }
}

impl HotKeyManagerDelegate for HotKeyManager {
    fn on_hot_key_pressed(&self, handle: HotKeyHandle) {
        let handle_to_isolate = self.handle_to_isolate.borrow();
        let isolate = handle_to_isolate.get(&handle);
        if let Some(isolate) = isolate {
            self.invoker
                .call_method(*isolate, "onHotKeyPressed", handle, |r| {
                    r.ok_log();
                });
        }
    }
    fn on_hot_key_released(&self, handle: HotKeyHandle) {
        let handle_to_isolate = self.handle_to_isolate.borrow();
        let isolate = handle_to_isolate.get(&handle);
        if let Some(isolate) = isolate {
            self.invoker
                .call_method(*isolate, "onHotKeyReleased", handle, |r| {
                    r.ok_log();
                });
        }
    }
}
