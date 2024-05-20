use std::{ffi::CString, mem::ManuallyDrop};

use objc2_app_kit::{NSBitmapImageRep, NSEvent, NSImage, NSView};
use objc2_foundation::{NSPoint, NSRect, NSSize};

use objc2::{
    class, declare::ClassBuilder, ffi::objc_getClass, msg_send, msg_send_id, rc::Id,
    runtime::AnyClass, ClassType, RefEncode,
};

use crate::{
    api_model::{ImageData, Point, Rect, Size},
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

impl From<Point> for NSPoint {
    fn from(point: Point) -> Self {
        NSPoint::new(point.x, point.y)
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

impl From<NSSize> for Size {
    fn from(s: NSSize) -> Self {
        Self {
            width: s.width,
            height: s.height,
        }
    }
}

pub(super) fn flip_rect(view: &NSView, rect: &mut NSRect) {
    let flipped: bool = view.isFlipped();
    if !flipped {
        rect.origin.y = view.bounds().size.height - rect.size.height - rect.origin.y;
    }
}

pub(super) fn flip_position(view: &NSView, position: &mut NSPoint) {
    let flipped: bool = view.isFlipped();
    if !flipped {
        position.y = view.bounds().size.height - position.y;
    }
}

struct MyClassBuilder {
    _cls: *mut AnyClass,
}

pub(super) fn class_builder_from_name(name: &str) -> ManuallyDrop<ClassBuilder> {
    let name = CString::new(name).unwrap();
    let class = unsafe { objc_getClass(name.as_ptr() as *const _) as *mut _ };
    let res = MyClassBuilder { _cls: class };
    // bit dirty, unfortunately ClassBuilder doesn't let us create instance with custom
    // class, and it's now worth replicating the entire functionality here
    ManuallyDrop::new(unsafe { std::mem::transmute::<MyClassBuilder, ClassBuilder>(res) })
}

enum _CGImage {}

unsafe impl RefEncode for _CGImage {
    const ENCODING_REF: objc2::Encoding =
        objc2::Encoding::Pointer(&objc2::Encoding::Struct("CGImage", &[]));
}

pub fn ns_image_from_image_data(images: Vec<ImageData>) -> Id<NSImage> {
    unsafe {
        let res = NSImage::init(NSImage::alloc());
        for image in images {
            let image = cg_image_from_image_data(image);
            let image = &*image as *const _ as *const _CGImage;
            let rep = NSBitmapImageRep::alloc();
            let rep: Id<NSBitmapImageRep> = msg_send_id![rep, initWithCGImage:image];
            res.addRepresentation(&rep);
        }
        res
    }
}

fn is_grayscale(image: &ImageData) -> bool {
    for pixel in image.data.chunks_exact(4) {
        if pixel[0] != pixel[1] || pixel[1] != pixel[2] {
            return false;
        }
    }
    true
}

pub fn ns_image_for_menu_item(image: ImageData) -> Id<NSImage> {
    let is_grayscale = is_grayscale(&image);
    let size = NSSize::new(image.point_width(), image.point_height());
    let image = ns_image_from_image_data(vec![image]);
    unsafe {
        image.setSize(size);
        image.setTemplate(is_grayscale);
    }
    image
}

enum _CGEvent {}

unsafe impl RefEncode for _CGEvent {
    const ENCODING_REF: objc2::Encoding =
        objc2::Encoding::Pointer(&objc2::Encoding::Struct("__CGEvent", &[]));
}

pub(crate) trait EventExt {
    #[allow(non_snake_case)]
    fn CGEvent(&self) -> core_graphics::sys::CGEventRef;
    #[allow(non_snake_case)]
    unsafe fn withCGEvent(event: core_graphics::sys::CGEventRef) -> Id<Self>;
}

impl EventExt for NSEvent {
    #[allow(non_snake_case)]
    fn CGEvent(&self) -> core_graphics::sys::CGEventRef {
        let event: *mut _CGEvent = unsafe { msg_send![self, CGEvent] };
        event as *mut _
    }

    #[allow(non_snake_case)]
    unsafe fn withCGEvent(event: core_graphics::sys::CGEventRef) -> Id<Self> {
        let res: Id<NSEvent> =
            msg_send_id![class!(NSEvent), eventWithCGEvent: event as * mut _CGEvent];
        res
    }
}
