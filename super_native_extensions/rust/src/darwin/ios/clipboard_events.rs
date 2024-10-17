use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use irondash_message_channel::Late;
use irondash_run_loop::{platform::PollSession, spawn, RunLoop};

use crate::clipboard_events_manager::{
    ClipboardEventManagerDelegate, PlatformClipboardEventManagerId,
};

thread_local! {
    pub static MANAGERS : RefCell<Vec<Weak<PlatformClipboardEventManager>>> = const { RefCell::new(Vec::new()) };
}

fn managers() -> Vec<Rc<PlatformClipboardEventManager>> {
    MANAGERS.with(|m| m.borrow().iter().filter_map(|m| m.upgrade()).collect())
}

pub struct PlatformClipboardEventManager {
    id: PlatformClipboardEventManagerId,
    delegate: Weak<dyn ClipboardEventManagerDelegate>,
    weak_self: Late<Weak<PlatformClipboardEventManager>>,
}

impl PlatformClipboardEventManager {
    pub fn new(
        id: PlatformClipboardEventManagerId,
        delegate: Weak<dyn ClipboardEventManagerDelegate>,
    ) -> Self {
        Self {
            id,
            delegate,
            weak_self: Late::new(),
        }
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformClipboardEventManager>) {
        self.weak_self.set(weak.clone());
        MANAGERS.with(|m| {
            m.borrow_mut().push(weak);
        });
    }

    async fn on_cut(&self) -> bool {
        let delegate = self.delegate.upgrade();
        match delegate {
            None => false,
            Some(delegate) => delegate.on_cut(self.id).await,
        }
    }

    async fn on_copy(&self) -> bool {
        let delegate = self.delegate.upgrade();
        match delegate {
            None => false,
            Some(delegate) => delegate.on_copy(self.id).await,
        }
    }

    async fn on_paste(&self) -> bool {
        let delegate = self.delegate.upgrade();
        match delegate {
            None => false,
            Some(delegate) => delegate.on_paste(self.id).await,
        }
    }

    async fn on_select_all(&self) -> bool {
        let delegate = self.delegate.upgrade();
        match delegate {
            None => false,
            Some(delegate) => delegate.on_select_all(self.id).await,
        }
    }
}

impl Drop for PlatformClipboardEventManager {
    fn drop(&mut self) {
        MANAGERS.with(|m| {
            let mut managers = m.borrow_mut();
            managers.retain(|m| m.as_ptr() != self.weak_self.as_ptr());
        });
    }
}

#[no_mangle]
pub extern "C" fn super_native_extensions_text_input_plugin_cut() -> bool {
    let mut handled = false;
    let managers = managers();
    for manager in managers {
        let done = Rc::new(Cell::new(None));
        let done2 = done.clone();
        spawn(async move {
            let handled = manager.on_cut().await;
            done2.set(Some(handled));
        });
        let mut poll_session = PollSession::new();
        while done.get().is_none() {
            RunLoop::current()
                .platform_run_loop
                .poll_once(&mut poll_session);
        }
        handled = handled || done.get().unwrap();
    }
    handled
}

#[no_mangle]
pub extern "C" fn super_native_extensions_text_input_plugin_copy() -> bool {
    let mut handled = false;
    let managers = managers();
    for manager in managers {
        let done = Rc::new(Cell::new(None));
        let done2 = done.clone();
        spawn(async move {
            let handled = manager.on_copy().await;
            done2.set(Some(handled));
        });
        let mut poll_session = PollSession::new();
        while done.get().is_none() {
            RunLoop::current()
                .platform_run_loop
                .poll_once(&mut poll_session);
        }
        handled = handled || done.get().unwrap();
    }
    handled
}

#[no_mangle]
pub extern "C" fn super_native_extensions_text_input_plugin_paste() -> bool {
    let mut handled = false;
    let managers = managers();
    for manager in managers {
        let done = Rc::new(Cell::new(None));
        let done2 = done.clone();
        spawn(async move {
            let handled = manager.on_paste().await;
            done2.set(Some(handled));
        });
        let mut poll_session = PollSession::new();
        while done.get().is_none() {
            RunLoop::current()
                .platform_run_loop
                .poll_once(&mut poll_session);
        }
        handled = handled || done.get().unwrap();
    }
    handled
}

#[no_mangle]
pub extern "C" fn super_native_extensions_text_input_plugin_select_all() -> bool {
    let mut handled = false;
    let managers = managers();
    for manager in managers {
        let done = Rc::new(Cell::new(None));
        let done2 = done.clone();
        spawn(async move {
            let handled = manager.on_select_all().await;
            done2.set(Some(handled));
        });
        let mut poll_session = PollSession::new();
        while done.get().is_none() {
            RunLoop::current()
                .platform_run_loop
                .poll_once(&mut poll_session);
        }
        handled = handled || done.get().unwrap();
    }
    handled
}
