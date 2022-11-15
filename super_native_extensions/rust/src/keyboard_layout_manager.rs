use std::{
    cell::RefCell,
    collections::HashSet,
    rc::{Rc, Weak},
};

use irondash_message_channel::{
    IntoValue, IsolateId, Late, MethodCall, MethodCallReply, MethodHandler, MethodInvoker,
    RegisteredMethodHandler, Value,
};

use crate::{context::Context, log::OkLog, platform_impl::platform::PlatformKeyboardLayout};

#[derive(IntoValue, Clone)]
#[irondash(rename_all = "camelCase")]
pub struct Key {
    pub platform: i64,
    pub physical: i64,
    pub logical: Option<i64>,
    pub logical_shift: Option<i64>,
    pub logical_alt: Option<i64>,
    pub logical_alt_shift: Option<i64>,
    pub logical_meta: Option<i64>,
}

#[derive(IntoValue, Clone)]
#[irondash(rename_all = "camelCase")]
pub struct KeyboardLayout {
    pub keys: Vec<Key>,
}

pub struct KeyboardLayoutManager {
    pub(crate) platform_layout: Late<Rc<PlatformKeyboardLayout>>,
    invoker: Late<MethodInvoker>,
    isolates: RefCell<HashSet<IsolateId>>,
}

pub trait KeyboardLayoutDelegate {
    fn keyboard_map_did_change(&self);
}

pub trait GetKeyboardLayoutDelegate {
    fn keyboard_map_manager(&self) -> Rc<KeyboardLayoutManager>;
}

impl GetKeyboardLayoutDelegate for Context {
    fn keyboard_map_manager(&self) -> Rc<KeyboardLayoutManager> {
        self.get_attachment(KeyboardLayoutManager::new).handler()
    }
}

impl KeyboardLayoutManager {
    pub fn new() -> RegisteredMethodHandler<Self> {
        Self {
            platform_layout: Late::new(),
            invoker: Late::new(),
            isolates: RefCell::new(HashSet::new()),
        }
        .register("KeyboardLayoutManager")
    }
}

impl MethodHandler for KeyboardLayoutManager {
    fn on_method_call(&self, call: MethodCall, reply: MethodCallReply) {
        #[allow(clippy::single_match)]
        match call.method.as_str() {
            "getKeyboardLayout" => {
                self.isolates.borrow_mut().insert(call.isolate);
                let layout = self.platform_layout.get_current_layout();
                reply.send_ok(layout);
            }
            _ => {}
        }
    }

    fn assign_weak_self(&self, weak_self: Weak<Self>) {
        let delegate: Weak<dyn KeyboardLayoutDelegate> = weak_self;
        self.platform_layout
            .set(Rc::new(PlatformKeyboardLayout::new(delegate)));
        self.platform_layout
            .assign_weak_self(Rc::downgrade(&self.platform_layout));
    }

    fn assign_invoker(&self, invoker: MethodInvoker) {
        self.invoker.set(invoker);
    }

    /// Called when isolate is about to be destroyed.
    fn on_isolate_destroyed(&self, isolate: IsolateId) {
        self.isolates.borrow_mut().remove(&isolate);
    }
}

impl KeyboardLayoutDelegate for KeyboardLayoutManager {
    fn keyboard_map_did_change(&self) {
        let layout = self.platform_layout.get_current_layout();
        let layout: Value = layout.into();
        let isolates = self.isolates.borrow();
        for isolate in isolates.iter() {
            self.invoker
                .call_method(*isolate, "onLayoutChanged", layout.clone(), |r| {
                    r.ok_log();
                });
        }
    }
}
