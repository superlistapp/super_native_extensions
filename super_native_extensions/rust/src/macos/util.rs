use std::{ffi::CString, mem::ManuallyDrop, os::raw::c_char, slice};

use cocoa::{
    appkit::NSView,
    base::{id, nil},
    foundation::{NSPoint, NSRect, NSSize, NSString},
};
use objc::{
    declare::ClassDecl,
    msg_send,
    runtime::{objc_getClass, Class, Object},
    sel, sel_impl, rc::StrongPtr,
};

use crate::api_model::Rect;

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

pub unsafe fn superclass(this: &Object) -> &Class {
    let superclass: id = msg_send![this, superclass];
    &*(superclass as *const _)
}

pub fn to_nsstring(string: &str) -> StrongPtr {
    unsafe {
        let ptr = NSString::alloc(nil).init_str(string);
        StrongPtr::new(ptr)
    }
}

pub unsafe fn from_nsstring(ns_string: id) -> String {
    let bytes: *const c_char = msg_send![ns_string, UTF8String];
    let bytes = bytes as *const u8;

    let len = ns_string.len();

    let bytes = slice::from_raw_parts(bytes, len);
    std::str::from_utf8(bytes).unwrap().into()
}
