use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    time::Duration,
};

use irondash_message_channel::Late;
use irondash_run_loop::{platform::MessageListener, RunLoop};
use windows::Win32::{
    Foundation::HWND,
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, MapVirtualKeyW, RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS,
            MAPVK_VSC_TO_VK, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN,
        },
        WindowsAndMessaging::WM_HOTKEY,
    },
};

use crate::{
    error::NativeExtensionsResult,
    hot_key_manager::{HotKeyCreateRequest, HotKeyHandle, HotKeyManagerDelegate},
};

// Virtual key codes for numpad keys (not affected by NumLock state issues)
const NUMPAD_VK_MAP: &[(i64, u32)] = &[
    // Numpad keys
    (71, 0x67), // Numpad7 -> VK_NUMPAD7
    (72, 0x68), // Numpad8 -> VK_NUMPAD8
    (73, 0x69), // Numpad9 -> VK_NUMPAD9
    (74, 0x6D), // NumpadSubtract -> VK_SUBTRACT
    (75, 0x64), // Numpad4 -> VK_NUMPAD4
    (76, 0x65), // Numpad5 -> VK_NUMPAD5
    (77, 0x66), // Numpad6 -> VK_NUMPAD6
    (78, 0x6B), // NumpadAdd -> VK_ADD
    (79, 0x61), // Numpad1 -> VK_NUMPAD1
    (80, 0x62), // Numpad2 -> VK_NUMPAD2
    (81, 0x63), // Numpad3 -> VK_NUMPAD3
    (82, 0x60), // Numpad0 -> VK_NUMPAD0
    (83, 0x6E), // NumpadDecimal -> VK_DECIMAL
    (55, 0x6A), // NumpadMultiply -> VK_MULTIPLY
];

pub struct PlatformHotKeyManager {
    delegate: Weak<dyn HotKeyManagerDelegate>,
    next_id: Cell<i32>,
    hot_keys: RefCell<HashMap<i32, (HotKeyHandle, HotKeyCreateRequest)>>,
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

    // Get virtual key code from platform code, using explicit mapping for numpad keys
    // to avoid issues with NumLock state affecting the conversion
    fn get_virtual_key(platform_code: i64) -> u32 {
        // Check if this is a numpad key with a known mapping
        for &(code, vk) in NUMPAD_VK_MAP {
            if code == platform_code {
                return vk;
            }
        }
        // For all other keys, use the standard conversion
        unsafe { MapVirtualKeyW(platform_code as u32, MAPVK_VSC_TO_VK) }
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
        modifiers |= MOD_NOREPEAT;
        let id = self.next_id.get();
        self.next_id.replace(id + 1);
        let vk = Self::get_virtual_key(request.platform_code);
        unsafe {
            RegisterHotKey(Self::hwnd(), id, modifiers, vk)?;
        }
        self.hot_keys.borrow_mut().insert(id, (handle, request));
        Ok(())
    }

    pub fn destroy_hot_key(&self, handle: HotKeyHandle) -> NativeExtensionsResult<()> {
        let mut hot_keys = self.hot_keys.borrow_mut();

        let hot_key_id = hot_keys
            .iter()
            .find(|(_, (h, _))| h == &handle)
            .map(|e| *e.0);
        if let Some(hot_key_id) = hot_key_id {
            hot_keys.remove(&hot_key_id);
            unsafe { UnregisterHotKey(Self::hwnd(), hot_key_id)? };
        }

        Ok(())
    }

    fn wait_until_release(
        request: HotKeyCreateRequest,
        handle: HotKeyHandle,
        delegate: Rc<dyn HotKeyManagerDelegate>,
    ) {
        let vk = Self::get_virtual_key(request.platform_code);
        let key_state = unsafe { GetAsyncKeyState(vk as i32) };
        if key_state < 0 {
            RunLoop::current()
                .schedule(Duration::from_millis(10), move || {
                    Self::wait_until_release(request, handle, delegate);
                })
                .detach();
        } else {
            delegate.on_hot_key_released(handle);
        }
    }

    fn on_hot_key(&self, hot_key: i32) {
        let hot_key = self.hot_keys.borrow().get(&hot_key).cloned();
        let delegate = self.delegate.upgrade();
        if let (Some((handle, request)), Some(delegate)) = (hot_key, delegate) {
            delegate.on_hot_key_pressed(handle);
            Self::wait_until_release(request, handle, delegate);
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
