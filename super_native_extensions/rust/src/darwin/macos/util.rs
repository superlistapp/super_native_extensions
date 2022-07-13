use std::{ffi::CString, mem::ManuallyDrop};

use cocoa::{
    appkit::{NSImage, NSView},
    base::{id, nil},
    foundation::{NSPoint, NSRect, NSSize},
};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    rc::StrongPtr,
    runtime::{objc_getClass, Class, Object},
    sel, sel_impl,
};

use crate::{
    api_model::{ImageData, Point, Rect},
    platform_impl::platform::common::cg_image_from_image_data,
};

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

impl From<NSPoint> for Point {
    fn from(point: NSPoint) -> Self {
        Self {
            x: point.x,
            y: point.y,
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

pub unsafe fn superclass(this: &Object) -> &Class {
    let superclass: id = msg_send![this, superclass];
    &*(superclass as *const _)
}

pub fn ns_image_from_image_data(images: Vec<ImageData>) -> StrongPtr {
    unsafe {
        let res = StrongPtr::new(msg_send![NSImage::alloc(nil), init]);
        for image in images {
            let image = cg_image_from_image_data(image);
            let rep: id = msg_send![class!(NSBitmapImageRep), alloc];
            let rep = StrongPtr::new(msg_send![rep, initWithCGImage:&*image]);
            NSImage::addRepresentation_(*res, *rep);
        }
        res
    }
}
