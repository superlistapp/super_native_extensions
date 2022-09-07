use std::rc::Weak;

use crate::keyboard_map_manager::{KeyboardLayoutDelegate, KeyboardMap};

pub struct PlatformKeyboardLayout {}

impl PlatformKeyboardLayout {
    pub fn new(_delegate: Weak<dyn KeyboardLayoutDelegate>) -> Self {
        Self {}
    }

    pub fn get_current_map(&self) -> Option<KeyboardMap> {
        None
    }

    pub fn assign_weak_self(&self, _weak: Weak<PlatformKeyboardLayout>) {}
}
