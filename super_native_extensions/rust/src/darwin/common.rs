use std::{os::raw::c_char, slice, sync::Arc};

use cocoa::{
    base::{id, nil},
    foundation::{NSDictionary, NSInteger, NSString},
};
use core_graphics::{
    base::{kCGBitmapByteOrderDefault, kCGImageAlphaLast, kCGRenderingIntentDefault, CGFloat},
    color_space::{kCGColorSpaceSRGB, CGColorSpace},
    data_provider::CGDataProvider,
    geometry::CGAffineTransform,
    image::CGImage,
};

use objc::{class, msg_send, rc::StrongPtr, sel, sel_impl};

use crate::api_model::ImageData;

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

pub fn to_nserror(domain: &str, code: NSInteger, message: &str) -> StrongPtr {
    unsafe {
        let user_info = NSDictionary::dictionaryWithObject_forKey_(
            nil,
            *to_nsstring(message),
            *to_nsstring("NSLocalizedDescription"),
        );
        let error: id = msg_send![class!(NSError), alloc];
        let error: id =
            msg_send![error, initWithDomain:to_nsstring(domain) code:code userInfo:user_info];
        StrongPtr::new(error)
    }
}

pub fn cg_image_from_image_data(image: ImageData) -> CGImage {
    let data = CGDataProvider::from_buffer(Arc::new(image.data));
    let rgb = CGColorSpace::create_with_name(unsafe { kCGColorSpaceSRGB })
        .unwrap_or_else(|| CGColorSpace::create_device_rgb());
    CGImage::new(
        image.width as usize,
        image.height as usize,
        8,
        32,
        image.bytes_per_row as usize,
        &rgb,
        kCGBitmapByteOrderDefault | kCGImageAlphaLast,
        &data,
        true,
        kCGRenderingIntentDefault,
    )
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    pub fn CGAffineTransformMakeScale(sx: CGFloat, sy: CGFloat) -> CGAffineTransform;
}
