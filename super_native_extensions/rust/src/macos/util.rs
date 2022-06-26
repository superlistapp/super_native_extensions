use std::{ffi::CString, mem::ManuallyDrop};

use cocoa::{
    appkit::NSView,
    base::id,
    foundation::{NSPoint, NSRect, NSSize},
};
use objc::{
    declare::ClassDecl,
    msg_send,
    runtime::{objc_getClass, Class},
    sel, sel_impl,
};

use crate::drag_drop_manager::Rect;

struct MyClassDecl {
    _cls: *mut Class,
}

pub(super) fn class_decl_from_name(name: &str) -> ManuallyDrop<ClassDecl> {
    let name = CString::new(name).unwrap();
    let class = unsafe { objc_getClass(name.as_ptr() as *const _) as *mut _ };
    let res = MyClassDecl { _cls: class };
    // bit dirty, unfortunatelly ClassDecl doesn't let us create instance with custom
    // class, and it's now worth replicating the entire functionality here
    ManuallyDrop::new(unsafe { std::mem::transmute(res) })
}

impl From<NSRect> for Rect {
    fn from(rect: NSRect) -> Self {
        Self {
            x: rect.origin.x,
            y: rect.origin.y,
            width: rect.size.width,
            height: rect.size.height,
        }
    }
}

impl From<Rect> for NSRect {
    fn from(rect: Rect) -> Self {
        NSRect::new(
            NSPoint::new(rect.x, rect.y),
            NSSize::new(rect.width, rect.height),
        )
    }
}

pub(super) unsafe fn flip_rect(view: id, rect: &mut NSRect) {
    let flipped: bool = msg_send![view, isFlipped];
    if !flipped {
        rect.origin.y = NSView::bounds(view).size.height - rect.size.height - rect.origin.y;
    }
}
