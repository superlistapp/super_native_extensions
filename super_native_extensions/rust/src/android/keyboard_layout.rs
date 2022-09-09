use std::rc::Weak;

use crate::keyboard_layout_manager::{KeyboardLayout, KeyboardLayoutDelegate};

pub struct PlatformKeyboardLayout {}

impl PlatformKeyboardLayout {
    pub fn new(_delegate: Weak<dyn KeyboardLayoutDelegate>) -> Self {
        Self {}
    }

    pub fn get_current_layout(&self) -> Option<KeyboardLayout> {
        None
    }

    pub fn assign_weak_self(&self, _weak: Weak<PlatformKeyboardLayout>) {}
}
