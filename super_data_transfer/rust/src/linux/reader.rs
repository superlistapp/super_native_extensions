use std::{
    ffi::CStr,
    os::raw::{c_char, c_int},
    rc::Weak,
    slice,
};

use gdk_sys::{gdk_display_get_default, GdkAtom};
use glib_sys::{gpointer, GFALSE};
use gobject_sys::{g_object_ref, g_object_unref};
use gtk_sys::{
    gtk_clipboard_get_default, gtk_clipboard_request_contents, gtk_clipboard_request_targets,
    gtk_clipboard_request_text, gtk_clipboard_request_uris, gtk_selection_data_get_data,
    gtk_selection_data_get_length, gtk_targets_include_text, GtkClipboard, GtkSelectionData,
};
use nativeshell_core::{
    util::{FutureCompleter, Late},
    Value,
};

use crate::error::ClipboardResult;

use super::{atom_from_string, atom_to_string, TYPE_TEXT, TYPE_URI};

pub struct PlatformClipboardReader {
    clipboard: *mut GtkClipboard,
    inner: Late<Inner>,
}

struct Inner {
    targets: Vec<String>,
    uris: Vec<String>,
}

impl PlatformClipboardReader {
    async fn init(&self) {
        if !self.inner.is_set() {
            let targets = self.get_targets().await;
            let uris = if targets.iter().any(|t| t == TYPE_URI) {
                self.get_uri_list().await
            } else {
                Vec::new()
            };
            // double check - we might have been preemted
            if !self.inner.is_set() {
                self.inner.set(Inner { targets, uris })
            }
        }
    }

    pub async fn get_items(&self) -> ClipboardResult<Vec<i64>> {
        self.init().await;
        // uris from urilist are represented as separate items
        let num_items = 1.max(self.inner.uris.len());
        Ok((0..num_items as i64).collect())
    }

    pub async fn get_types_for_item(&self, item: i64) -> ClipboardResult<Vec<String>> {
        self.init().await;
        if item == 0 {
            Ok(self.inner.targets.clone())
        } else if (item as usize) < self.inner.uris.len() {
            Ok(vec![TYPE_URI.into()])
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn get_data_for_item(&self, item: i64, data_type: String) -> ClipboardResult<Value> {
        let item = item as usize;
        if data_type == TYPE_URI && item < self.inner.uris.len() {
            Ok(self.inner.uris[item].clone().into())
        } else if item == 0 {
            let mut target = atom_from_string(&data_type);
            let is_text = unsafe { gtk_targets_include_text(&mut target as *mut _, 1) } != GFALSE;
            if is_text {
                Ok(self.get_text().await.into())
            } else {
                Ok(self.get_data(&data_type).await.into())
            }
        } else {
            Ok(Value::Null)
        }
    }

    pub fn new_default() -> ClipboardResult<Self> {
        let clipboard = unsafe {
            let display = gdk_display_get_default();
            let clipboard = gtk_clipboard_get_default(display);
            g_object_ref(clipboard as *mut _) as *mut GtkClipboard
        };
        Ok(PlatformClipboardReader {
            clipboard,
            inner: Late::new(),
        })
    }

    pub fn assign_weak_self(&self, _weak: Weak<PlatformClipboardReader>) {}

    async fn get_targets(&self) -> Vec<String> {
        let (future, completer) = FutureCompleter::new();
        unsafe {
            gtk_clipboard_request_targets(
                self.clipboard,
                Some(Self::on_targets),
                Box::into_raw(Box::new(completer)) as *mut _,
            )
        };
        future.await
    }

    async fn get_text(&self) -> Option<String> {
        let (future, completer) = FutureCompleter::new();
        unsafe {
            gtk_clipboard_request_text(
                self.clipboard,
                Some(Self::on_text),
                Box::into_raw(Box::new(completer)) as *mut _,
            )
        }
        future.await
    }

    async fn get_uri_list(&self) -> Vec<String> {
        let (future, completer) = FutureCompleter::new();
        unsafe {
            gtk_clipboard_request_uris(
                self.clipboard,
                Some(Self::on_uri_list),
                Box::into_raw(Box::new(completer)) as *mut _,
            )
        }
        future.await
    }

    async fn get_data(&self, ty: &str) -> Option<Vec<u8>> {
        let (future, completer) = FutureCompleter::new();
        unsafe {
            gtk_clipboard_request_contents(
                self.clipboard,
                atom_from_string(ty),
                Some(Self::on_contents),
                Box::into_raw(Box::new(completer)) as *mut _,
            )
        }
        future.await
    }

    extern "C" fn on_targets(
        _: *mut GtkClipboard,
        targets: *mut GdkAtom,
        n_targets: c_int,
        completer: gpointer,
    ) {
        let has_text = unsafe { gtk_targets_include_text(targets, n_targets) } != GFALSE;
        let targets = unsafe { slice::from_raw_parts(targets, n_targets as usize) };
        let completer = completer as *mut FutureCompleter<Vec<String>>;
        let completer = unsafe { Box::from_raw(completer) };
        let mut targets: Vec<_> = targets
            .iter()
            .map(|t| unsafe { atom_to_string(t) })
            .collect();
        // Ensure we report our TEXT_TYPE
        if has_text && !targets.iter().any(|a| a == TYPE_TEXT) {
            targets.push(TYPE_TEXT.to_owned());
        }
        completer.complete(targets);
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
}

impl Drop for PlatformClipboardReader {
    fn drop(&mut self) {
        unsafe {
            g_object_unref(self.clipboard as *mut _);
        }
    }
}
