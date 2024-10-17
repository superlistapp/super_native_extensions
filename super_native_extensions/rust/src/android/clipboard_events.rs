use crate::clipboard_events_manager::{
    ClipboardEventManagerDelegate, PlatformClipboardEventManagerId,
};
use std::rc::Weak;

pub struct PlatformClipboardEventManager {}

impl PlatformClipboardEventManager {
    pub fn new(
        _id: PlatformClipboardEventManagerId,
        _delegate: Weak<dyn ClipboardEventManagerDelegate>,
    ) -> Self {
        Self {}
    }

    pub fn assign_weak_self(&self, _weak: Weak<PlatformClipboardEventManager>) {}
}
