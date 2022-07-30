use std::rc::Weak;

use gdk::glib::{translate::from_glib_none, WeakRef};
use gobject_sys::GObject;
use gtk::Widget;
use gtk_sys::GtkWidget;

use crate::{
    drop_manager::{PlatformDropContextDelegate, PlatformDropContextId},
    error::NativeExtensionsResult,
};

pub struct PlatformDropContext {
    id: PlatformDropContextId,
    view: WeakRef<Widget>,
    delegate: Weak<dyn PlatformDropContextDelegate>,
}

impl PlatformDropContext {
    pub fn new(id: i64, view_handle: i64, delegate: Weak<dyn PlatformDropContextDelegate>) -> Self {
        let view: Widget = unsafe { from_glib_none(view_handle as *mut GtkWidget) };
        let mut weak = WeakRef::new();
        weak.set(Some(&view));

        Self {
            id,
            view: weak,
            delegate,
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {}

    pub fn register_drop_types(&self, types: &[String]) -> NativeExtensionsResult<()> {
        Ok(())
    }
}
