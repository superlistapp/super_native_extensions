use std::ffi::{CStr, CString};

use gdk::{
    cairo::{Format, ImageSurface},
    glib::translate::{FromGlibPtrNone, ToGlibPtr, ToGlibPtrMut},
    Atom, Event, EventType,
};
use gdk_sys::{gdk_atom_intern, gdk_atom_name, GdkAtom};
use glib_sys::GFALSE;
use gtk::{TargetEntry, TargetList};
use gtk_sys::{gtk_target_table_new_from_list, gtk_targets_include_text};

use crate::api_model::ImageData;

// Use gtk function to set/retrieve text (there are multiple possible format,
// we don't want to mess with that)
pub const TYPE_TEXT: &str = "text/plain";

// Special care for URIs. When writing URIs from multiple items are merged into one
// URI list, when reading URI list is split into multiple items.
pub const TYPE_URI: &str = "text/uri-list";

pub trait AtomExt {
    fn from_string(s: &str) -> GdkAtom;
    fn to_string(&self) -> String;
}

impl AtomExt for GdkAtom {
    fn from_string(s: &str) -> GdkAtom {
        let s = CString::new(s).unwrap();
        unsafe { gdk_atom_intern(s.as_ptr(), GFALSE) }
    }

    fn to_string(&self) -> String {
        unsafe {
            let s = gdk_atom_name(*self);
            CStr::from_ptr(s).to_string_lossy().into()
        }
    }
}

pub fn target_includes_text(target: &Atom) -> bool {
    let res = unsafe { gtk_targets_include_text(&mut target.to_glib_none().0, 1) };
    res != GFALSE
}

pub trait TargetListExt {
    fn get_target_entries(&self) -> Vec<TargetEntry>;
}

impl TargetListExt for TargetList {
    fn get_target_entries(&self) -> Vec<TargetEntry> {
        let mut n_targets = 0;
        let targets =
            unsafe { gtk_target_table_new_from_list(self.to_glib_none().0, &mut n_targets) };
        let mut entries = Vec::<TargetEntry>::new();
        for i in 0..n_targets as usize {
            entries.push(unsafe { TargetEntry::from_glib_none(targets.add(i)) })
        }
        entries
    }
}

pub fn surface_from_image_data(image: ImageData, opacity: f64) -> ImageSurface {
    let factor: i32 = (opacity * 255.0) as i32;

    let mut data = image.data;
    for offset in (0..data.len()).step_by(4) {
        let (r, g, b, a) = (
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        );

        let a = if factor == 255 {
            a
        } else {
            (a as i32 * factor / 255) as u8
        };

        // Premultiply alpha
        data[offset] = (b as i32 * a as i32 / 255) as u8;
        data[offset + 1] = (g as i32 * a as i32 / 255) as u8;
        data[offset + 2] = (r as i32 * a as i32 / 255) as u8;
        data[offset + 3] = a;
    }
    let surface = ImageSurface::create_for_data(
        data,
        Format::ARgb32,
        image.width,
        image.height,
        image.bytes_per_row,
    );
    let res = surface.unwrap();
    res.set_device_scale(
        image.device_pixel_ratio.unwrap_or(1.0),
        image.device_pixel_ratio.unwrap_or(1.0),
    );
    res
}

pub(super) fn synthesize_button_up(event: &Event) -> Event {
    if event.event_type() != EventType::ButtonPress {
        panic!("Invalid event type");
    }
    let mut event = event.clone();
    let e: *mut gdk_sys::GdkEvent = event.to_glib_none_mut().0;
    let e = unsafe { &mut *e };
    e.type_ = gdk_sys::GDK_BUTTON_RELEASE;
    event
}
