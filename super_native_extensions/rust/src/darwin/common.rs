use std::{
    ffi::{CStr, OsStr},
    os::unix::prelude::OsStrExt,
    path::PathBuf,
    sync::Arc,
};

use core_foundation::{
    base::{Boolean, TCFType},
    string::{CFString, CFStringRef},
};
use core_graphics::{
    base::{kCGBitmapByteOrderDefault, kCGImageAlphaLast, kCGRenderingIntentDefault},
    color_space::{kCGColorSpaceSRGB, CGColorSpace},
    data_provider::CGDataProvider,
    image::CGImage,
};
use objc2::{ffi::NSInteger, rc::Id, runtime::AnyObject, ClassType};
use objc2_foundation::{ns_string, NSDictionary, NSError, NSString, NSURLTypeIdentifierKey, NSURL};

use crate::api_model::ImageData;

pub struct NSURLSecurtyScopeAccess {
    url: Id<NSURL>,
}

impl NSURLSecurtyScopeAccess {
    pub fn new(url: &NSURL) -> Self {
        // Necessary for iOS to access files outside of the app's sandbox.
        // However on macOS it breaks accessing fileURLs in pasteboard.
        #[cfg(target_os = "ios")]
        unsafe {
            url.startAccessingSecurityScopedResource();
        }
        Self { url: url.retain() }
    }
}

impl Drop for NSURLSecurtyScopeAccess {
    fn drop(&mut self) {
        #[cfg(target_os = "ios")]
        unsafe {
            self.url.stopAccessingSecurityScopedResource();
        }
    }
}

pub fn to_nserror(domain: &str, code: NSInteger, message: &str) -> Id<NSError> {
    unsafe {
        let user_info = NSDictionary::<NSString, AnyObject>::from_vec(
            &[ns_string!("NSLocalizedDescription")],
            vec![Id::cast(NSString::from_str(message))],
        );

        NSError::errorWithDomain_code_userInfo(&NSString::from_str(domain), code, Some(&user_info))
    }
}

pub fn path_from_url(url: &NSURL) -> PathBuf {
    let path: *const i8 = unsafe { url.fileSystemRepresentation() }.as_ptr();
    let path = unsafe { CStr::from_ptr(path) };
    let path = OsStr::from_bytes(path.to_bytes());
    path.into()
}

pub unsafe fn format_from_url(url: &NSURL) -> Option<String> {
    let mut ty: Option<Id<AnyObject>> = None;
    let _access = NSURLSecurtyScopeAccess::new(url);
    let res = url.getResourceValue_forKey_error(&mut ty, NSURLTypeIdentifierKey);
    if let (Some(ty), Ok(_)) = (ty, res) {
        Some(Id::cast::<NSString>(ty).to_string())
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
