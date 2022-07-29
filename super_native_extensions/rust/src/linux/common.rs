use std::ffi::{CString, CStr};

use gdk_sys::{GdkAtom, gdk_atom_intern, gdk_atom_name};
use glib_sys::GFALSE;

// Use gtk function to set/retrieve text (there are multiple possible format,
// we don't want to mess with that)
pub const TYPE_TEXT: &str = "text/plain";

// Special care for URIs. When writing URIs from multiple items are merged into one
// URI list, when reading URI list is split into multiple items.
pub const TYPE_URI: &str = "text/uri-list";

pub fn atom_from_string(s: &str) -> GdkAtom {
    let s = CString::new(s).unwrap();
    unsafe { gdk_atom_intern(s.as_ptr(), GFALSE) }
}

pub unsafe fn atom_to_string(atom: &GdkAtom) -> String {
    let s = gdk_atom_name(*atom);
    CStr::from_ptr(s).to_string_lossy().into()
}
