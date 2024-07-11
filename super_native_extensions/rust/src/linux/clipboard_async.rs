use std::{
    ffi::CStr,
    os::raw::{c_char, c_int},
    slice,
};

use async_trait::async_trait;
use gdk::glib::translate::ToGlibPtr;
use gdk_sys::GdkAtom;
use glib_sys::{gpointer, GFALSE};
use gtk::Clipboard;
use gtk_sys::{
    gtk_clipboard_request_contents, gtk_clipboard_request_targets, gtk_clipboard_request_text,
    gtk_clipboard_request_uris, gtk_selection_data_get_data, gtk_selection_data_get_length,
    gtk_targets_include_text, GtkClipboard, GtkSelectionData,
};
use irondash_run_loop::util::FutureCompleter;

use super::common::{AtomExt, TYPE_TEXT};

#[async_trait(?Send)]
pub trait ClipboardAsync {
    async fn get_targets(&self) -> Vec<String>;
    async fn get_text(&self) -> Option<String>;
    async fn get_uri_list(&self) -> Vec<String>;
    async fn get_data(&self, ty: &str) -> Option<Vec<u8>>;
}

#[async_trait(?Send)]
impl ClipboardAsync for Clipboard {
    async fn get_targets(&self) -> Vec<String> {
        let (future, completer) = FutureCompleter::new();
        unsafe {
            gtk_clipboard_request_targets(
                self.to_glib_none().0,
                Some(on_targets),
                Box::into_raw(Box::new(completer)) as *mut _,
            )
        };
        future.await
    }

    async fn get_text(&self) -> Option<String> {
        let (future, completer) = FutureCompleter::new();
        unsafe {
            gtk_clipboard_request_text(
                self.to_glib_none().0,
                Some(on_text),
                Box::into_raw(Box::new(completer)) as *mut _,
            )
        }
        future.await
    }

    async fn get_uri_list(&self) -> Vec<String> {
        let (future, completer) = FutureCompleter::new();
        unsafe {
            gtk_clipboard_request_uris(
                self.to_glib_none().0,
                Some(on_uri_list),
                Box::into_raw(Box::new(completer)) as *mut _,
            )
        }
        future.await
    }

    async fn get_data(&self, ty: &str) -> Option<Vec<u8>> {
        let (future, completer) = FutureCompleter::new();
        unsafe {
            gtk_clipboard_request_contents(
                self.to_glib_none().0,
                GdkAtom::from_string(ty),
                Some(on_contents),
                Box::into_raw(Box::new(completer)) as *mut _,
            )
        }
        future.await
    }
}

extern "C" fn on_targets(
    _: *mut GtkClipboard,
    targets: *mut GdkAtom,
    n_targets: c_int,
    completer: gpointer,
) {
    let completer = completer as *mut FutureCompleter<Vec<String>>;
    let completer = unsafe { Box::from_raw(completer) };
    if n_targets > 0 {
        let has_text = unsafe { gtk_targets_include_text(targets, n_targets) } != GFALSE;
        let targets = unsafe { slice::from_raw_parts(targets, n_targets as usize) };
        let mut targets: Vec<_> = targets.iter().map(|t| t.to_string()).collect();
        // Ensure we report our TEXT_TYPE
        if has_text && !targets.iter().any(|a| a == TYPE_TEXT) {
            targets.push(TYPE_TEXT.to_owned());
        }
        completer.complete(targets);
    } else {
        completer.complete(vec![]);
    }
}

extern "C" fn on_text(_: *mut GtkClipboard, text: *const c_char, completer: gpointer) {
    let completer = completer as *mut FutureCompleter<Option<String>>;
    let completer = unsafe { Box::from_raw(completer) };
    if text.is_null() {
        completer.complete(None)
    } else {
        let text = unsafe { CStr::from_ptr(text) }.to_string_lossy();
        completer.complete(Some(text.into()))
    }
}

extern "C" fn on_uri_list(_: *mut GtkClipboard, list: *mut *mut c_char, completer: gpointer) {
    let completer = completer as *mut FutureCompleter<Vec<String>>;
    let completer = unsafe { Box::from_raw(completer) };
    let mut res = Vec::new();
    if !list.is_null() {
        let mut index = 0usize;
        loop {
            let uri = unsafe { *list.add(index) };
            if uri.is_null() {
                break;
            } else {
                let uri = unsafe { CStr::from_ptr(uri) }.to_string_lossy();
                res.push(uri.into());
            }
            index += 1;
        }
    }
    completer.complete(res)
}

extern "C" fn on_contents(
    _: *mut GtkClipboard,
    selection_data: *mut GtkSelectionData,
    completer: gpointer,
) {
    let completer = completer as *mut FutureCompleter<Option<Vec<u8>>>;
    let completer = unsafe { Box::from_raw(completer) };
    if selection_data.is_null() {
        completer.complete(None);
    } else {
        let data = unsafe {
            let data = gtk_selection_data_get_data(selection_data);
            if data.is_null() {
                None
            } else {
                let len = gtk_selection_data_get_length(selection_data) as usize;
                let s = slice::from_raw_parts(data, len);
                Some(s.to_owned())
            }
        };
        completer.complete(data);
    }
}
