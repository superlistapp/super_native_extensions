use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    mem,
    rc::Weak,
};

use core_foundation::base::OSStatus;
use irondash_message_channel::Late;
use log::warn;

use crate::{
    error::NativeExtensionsResult,
    hot_key_manager::{HotKeyCreateRequest, HotKeyHandle, HotKeyManagerDelegate},
};

use super::hot_key_sys::{
    kEventClassKeyboard, kEventHotKeyPressed, kEventHotKeyReleased, kEventParamDirectObject,
    typeEventHotKeyID, EventHandlerCallRef, EventHandlerRef, EventHotKeyID, EventHotKeyRef,
    EventRef, EventTypeSpec, GetEventDispatcherTarget, GetEventKind, GetEventParameter,
    InstallEventHandler, RegisterEventHotKey, RemoveEventHandler, UnregisterEventHotKey,
};

const HOT_KEY_TAG: u32 = 1314080844; // NSHL

struct HotKey {
    handle: HotKeyHandle,
    key_ref: EventHotKeyRef,
}
pub struct PlatformHotKeyManager {
    delegate: Weak<dyn HotKeyManagerDelegate>,
    weak_self: Late<Weak<PlatformHotKeyManager>>,
    event_handler_ref: Cell<EventHandlerRef>,
    next_id: Cell<u32>,
    hot_keys: RefCell<HashMap<u32, HotKey>>,
}

impl PlatformHotKeyManager {
    pub fn new(delegate: Weak<dyn HotKeyManagerDelegate>) -> Self {
        Self {
            delegate,
            weak_self: Late::new(),
            event_handler_ref: Cell::new(std::ptr::null_mut()),
            next_id: Cell::new(1),
            hot_keys: RefCell::new(HashMap::new()),
        }
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformHotKeyManager>) {
        self.weak_self.set(weak.clone());

        let spec = [
            EventTypeSpec {
                eventClass: kEventClassKeyboard,
                eventKind: kEventHotKeyPressed,
            },
            EventTypeSpec {
                eventClass: kEventClassKeyboard,
                eventKind: kEventHotKeyReleased,
            },
        ];

        let ptr = Box::into_raw(Box::new(weak));
        let mut event_handler_ref: EventHandlerRef = std::ptr::null_mut();

        let status = unsafe {
            InstallEventHandler(
                GetEventDispatcherTarget(),
                Some(event_handler),
                2,
                &spec as *const _,
                ptr as *mut _,
                &mut event_handler_ref as *mut _,
            )
        };
        self.event_handler_ref.replace(event_handler_ref);
        if status != 0 {
            warn!("Couldn't install event handler: {}", status);
        }
    }

    fn on_hot_key_pressed(&self, hot_key_id: u32) {
        if let Some(key) = self.hot_keys.borrow().get(&hot_key_id) {
            if let Some(delegate) = self.delegate.upgrade() {
                delegate.on_hot_key_pressed(key.handle);
            }
        }
    }

    fn on_hot_key_released(&self, hot_key_id: u32) {
        if let Some(key) = self.hot_keys.borrow().get(&hot_key_id) {
            if let Some(delegate) = self.delegate.upgrade() {
                delegate.on_hot_key_released(key.handle);
            }
        }
    }

    pub fn create_hot_key(
        &self,
        handle: HotKeyHandle,
        request: HotKeyCreateRequest,
    ) -> NativeExtensionsResult<()> {
        let id = self.next_id.get();
        self.next_id.replace(id + 1);

        let hot_key_id = EventHotKeyID {
            signature: HOT_KEY_TAG,
            id,
        };

        let mut modifiers = 0u32;
        if request.meta {
            modifiers |= 1 << 8;
        }
        if request.shift {
            modifiers |= 1 << 9;
        }
        if request.alt {
            modifiers |= 1 << 11;
        }
        if request.control {
            modifiers |= 1 << 12;
        }

        let mut key_ref: EventHotKeyRef = std::ptr::null_mut();

        unsafe {
            RegisterEventHotKey(
                request.platform_code as u32,
                modifiers,
                hot_key_id,
                GetEventDispatcherTarget(),
                0,
                &mut key_ref as *mut _,
            );
        };

        let key = HotKey { handle, key_ref };

        self.hot_keys.borrow_mut().insert(id, key);

        Ok(())
    }

    pub fn destroy_hot_key(&self, handle: HotKeyHandle) -> NativeExtensionsResult<()> {
        let mut hot_keys = self.hot_keys.borrow_mut();

        let hot_key_id = hot_keys.iter().find(|f| f.1.handle == handle).map(|e| *e.0);
        let hot_key = hot_key_id.and_then(|id| hot_keys.remove(&id));

        if let Some(hot_key) = hot_key {
            unsafe {
                UnregisterEventHotKey(hot_key.key_ref);
            }
        }

        Ok(())
    }
}

impl Drop for PlatformHotKeyManager {
    fn drop(&mut self) {
        if !self.event_handler_ref.get().is_null() {
            unsafe { RemoveEventHandler(self.event_handler_ref.get()) };
        }
    }
}

unsafe extern "C" fn event_handler(
    _in_handler_call_ref: EventHandlerCallRef,
    in_event: EventRef,
    in_user_data: *mut ::std::os::raw::c_void,
) -> OSStatus {
    let mut hot_key_id = EventHotKeyID {
        signature: 0,
        id: 0,
    };
    #[allow(clippy::collapsible_if)]
    if GetEventParameter(
        in_event,
        kEventParamDirectObject,
        typeEventHotKeyID,
        std::ptr::null_mut(),
        mem::size_of::<EventHotKeyID>() as u64,
        std::ptr::null_mut(),
        &mut hot_key_id as *mut _ as *mut _,
    ) == 0
    {
        if hot_key_id.signature == HOT_KEY_TAG {
            let kind = GetEventKind(in_event);
            let manager = in_user_data as *mut Weak<PlatformHotKeyManager>;
            let manager = &*manager;
            if let Some(manager) = manager.upgrade() {
                if kind == kEventHotKeyPressed {
                    manager.on_hot_key_pressed(hot_key_id.id);
                } else if kind == kEventHotKeyReleased {
                    manager.on_hot_key_released(hot_key_id.id);
                }
            }
        }
    }
    0
}
