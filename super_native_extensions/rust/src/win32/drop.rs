use std::rc::Weak;

use nativeshell_core::util::Late;
use windows::Win32::Foundation::HWND;

use crate::{drop_manager::PlatformDropContextDelegate, error::NativeExtensionsResult};

pub struct PlatformDropContext {
    id: i64,
    weak_self: Late<Weak<Self>>,
    view: HWND,
    delegate: Weak<dyn PlatformDropContextDelegate>,
}

impl PlatformDropContext {
    pub fn new(id: i64, view_handle: i64, delegate: Weak<dyn PlatformDropContextDelegate>) -> Self {
        Self {
            id,
            weak_self: Late::new(),
            view: HWND(view_handle as isize),
            delegate,
        }
    }

    pub fn register_drop_types(&self, types: &[String]) -> NativeExtensionsResult<()> {
        Ok(())
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self.clone());
    }
}
