use std::{
    ffi::{CStr, OsStr},
    os::{
        raw::{c_char, c_void},
        unix::prelude::OsStrExt,
    },
    path::PathBuf,
    slice,
    sync::Arc,
};

use cocoa::{
    base::{id, nil, BOOL, YES},
    foundation::{NSDictionary, NSInteger, NSString},
};
use core_foundation::{
    base::{Boolean, TCFType},
    string::{CFString, CFStringRef},
};
use core_graphics::{
    base::{kCGBitmapByteOrderDefault, kCGImageAlphaLast, kCGRenderingIntentDefault, CGFloat},
    color_space::{kCGColorSpaceSRGB, CGColorSpace},
    data_provider::CGDataProvider,
    geometry::CGAffineTransform,
    image::CGImage,
};

use objc::{class, msg_send, rc::StrongPtr, runtime::Object, sel, sel_impl};

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

    let mut bytes = slice::from_raw_parts(bytes, len);
    while bytes.last() == Some(&0) {
        bytes = &bytes[..bytes.len() - 1];
    }
    std::str::from_utf8(bytes).unwrap().into()
}

pub fn to_nsdata(data: &[u8]) -> StrongPtr {
    unsafe {
        let d: id = msg_send![class!(NSData), alloc];
        let d: id = msg_send![d, initWithBytes:data.as_ptr() length:data.len()];
        StrongPtr::new(d)
    }
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

pub fn nserror_description(error: id) -> String {
    unsafe {
        let description: id = msg_send![error, localizedDescription];
        from_nsstring(description)
    }
}

pub fn path_from_url(url: id) -> PathBuf {
    let path: *const i8 = unsafe { msg_send![url, fileSystemRepresentation] };
    let path = unsafe { CStr::from_ptr(path) };
    let path = OsStr::from_bytes(path.to_bytes());
    path.into()
}

pub unsafe fn format_from_url(url: id) -> Option<String> {
    let ty: id = std::ptr::null_mut();
    let res: BOOL = msg_send![url, getResourceValue:&ty forKey:NSURLTypeIdentifierKey error:nil];
    if res == YES && !ty.is_null() {
        Some(from_nsstring(ty))
    } else {
        None
    }
}

pub fn cg_image_from_image_data(image: ImageData) -> CGImage {
    let data = CGDataProvider::from_buffer(Arc::new(image.data));
    let rgb = CGColorSpace::create_with_name(unsafe { kCGColorSpaceSRGB })
        .unwrap_or_else(CGColorSpace::create_device_rgb);
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

#[allow(non_camel_case_types)]
pub type objc_AssociationPolicy = usize;
pub const OBJC_ASSOCIATION_ASSIGN: objc_AssociationPolicy = 0;
pub const OBJC_ASSOCIATION_RETAIN_NONATOMIC: objc_AssociationPolicy = 1;
pub const OBJC_ASSOCIATION_COPY_NONATOMIC: objc_AssociationPolicy = 3;
pub const OBJC_ASSOCIATION_RETAIN: objc_AssociationPolicy = 769;
pub const OBJC_ASSOCIATION_COPY: objc_AssociationPolicy = 771;

extern "C" {
    pub fn objc_setAssociatedObject(
        object: *mut Object,
        key: *const c_void,
        value: *mut Object,
        policy: objc_AssociationPolicy,
    );
}

extern "C" {
    pub static NSURLTypeIdentifierKey: id;
}

#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    pub fn UTTypeConformsTo(name: CFStringRef, inConformsToUTI: CFStringRef) -> Boolean;
}

pub fn uti_conforms_to(uti: &str, conforms_to: &str) -> bool {
    let uti = CFString::new(uti);
    let conforms_to = CFString::new(conforms_to);

    let conforms_to =
        unsafe { UTTypeConformsTo(uti.as_concrete_TypeRef(), conforms_to.as_concrete_TypeRef()) };

    conforms_to != 0
}
