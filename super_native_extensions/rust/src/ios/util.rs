use std::{os::raw::c_char, slice};

use cocoa::{
    base::{id, nil},
    foundation::{NSDictionary, NSInteger, NSString},
};
use objc::{class, msg_send, rc::StrongPtr, sel, sel_impl};

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
