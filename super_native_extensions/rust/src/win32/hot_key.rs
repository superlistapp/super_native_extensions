use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Weak,
};

use irondash_message_channel::Late;
use irondash_run_loop::{platform::MessageListener, RunLoop};
use windows::Win32::{
    Foundation::HWND,
    UI::{
        Input::KeyboardAndMouse::{
            MapVirtualKeyW, RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MAPVK_VSC_TO_VK,
            MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN,
        },
        WindowsAndMessaging::WM_HOTKEY,
    },
};

use crate::{
    error::NativeExtensionsResult,
    hot_key_manager::{HotKeyCreateRequest, HotKeyHandle, HotKeyManagerDelegate},
};

pub struct PlatformHotKeyManager {
    delegate: Weak<dyn HotKeyManagerDelegate>,
    next_id: Cell<i32>,
    hot_keys: RefCell<HashMap<i32, HotKeyHandle>>,
    weak_self: Late<Weak<Self>>,
}

impl PlatformHotKeyManager {
    pub fn new(delegate: Weak<dyn HotKeyManagerDelegate>) -> Self {
        Self {
            delegate,
            next_id: Cell::new(65536),
            hot_keys: RefCell::new(HashMap::new()),
            weak_self: Late::new(),
        }
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformHotKeyManager>) {
        self.weak_self.set(weak.clone());
        RunLoop::current()
            .platform_run_loop
            .register_message_listener(weak);
    }

    fn hwnd() -> HWND {
        HWND(RunLoop::current().platform_run_loop.hwnd())
    }

    pub fn create_hot_key(
        &self,
        handle: HotKeyHandle,
        request: HotKeyCreateRequest,
    ) -> NativeExtensionsResult<()> {
        let mut modifiers = HOT_KEY_MODIFIERS::default();
        if request.alt {
            modifiers |= MOD_ALT;
        }
        if request.control {
            modifiers |= MOD_CONTROL;
        }
        if request.shift {
            modifiers |= MOD_SHIFT;
        }
        if request.meta {
            modifiers |= MOD_WIN;
        }
        let id = self.next_id.get();
        self.next_id.replace(id + 1);
        self.hot_keys.borrow_mut().insert(id, handle);
        unsafe {
            let vk = MapVirtualKeyW(request.platform_code as u32, MAPVK_VSC_TO_VK);
            RegisterHotKey(Self::hwnd(), id, modifiers, vk);
        }
        Ok(())
    }

    pub fn destroy_hot_key(&self, handle: HotKeyHandle) -> NativeExtensionsResult<()> {
        let mut hot_keys = self.hot_keys.borrow_mut();

        let hot_key_id = hot_keys.iter().find(|f| f.1 == &handle).map(|e| *e.0);
        if let Some(hot_key_id) = hot_key_id {
            hot_keys.remove(&hot_key_id);
            unsafe { UnregisterHotKey(Self::hwnd(), hot_key_id) };
        }

        Ok(())
    }

    fn on_hot_key(&self, hot_key: i32) {
        let handle = self.hot_keys.borrow().get(&hot_key).cloned();
        let delegate = self.delegate.upgrade();
        if let (Some(handle), Some(delegate)) = (handle, delegate) {
            delegate.on_hot_key_pressed(handle);
        }
    }
}

impl Drop for PlatformHotKeyManager {
    fn drop(&mut self) {
        let message_listener: Weak<dyn MessageListener> = self.weak_self.clone();
        if let Ok(run_loop) = RunLoop::try_current() {
            run_loop
                .platform_run_loop
                .unregister_message_listener(&message_listener);
        }
    }
}

impl MessageListener for PlatformHotKeyManager {
    fn on_window_message(&self, _hwnd: isize, message: u32, w_param: usize, _l_param: isize) {
        if message == WM_HOTKEY {
            self.on_hot_key(w_param as _)
        }
    }
}
