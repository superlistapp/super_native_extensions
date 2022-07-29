use std::{
    mem::ManuallyDrop,
    os::raw::{c_int, c_uint},
    ptr::null_mut,
    rc::{Rc, Weak},
    sync::Arc,
};

use gdk_sys::gdk_display_get_default;
use glib_sys::{gpointer, GFALSE};
use gtk_sys::{
    gtk_clipboard_get_default, gtk_clipboard_set_with_data, gtk_main_iteration,
    gtk_selection_data_get_target, gtk_selection_data_set, gtk_selection_data_set_text,
    gtk_target_list_add, gtk_target_list_add_text_targets, gtk_target_list_new,
    gtk_target_list_unref, gtk_target_table_free, gtk_target_table_new_from_list,
    gtk_targets_include_text, GtkClipboard, GtkSelectionData, GtkTargetList,
};
use nativeshell_core::{util::Late, IsolateId};
use scopeguard::defer;

use crate::{
    api_model::{DataProvider, DataRepresentation},
    data_provider_manager::{DataProviderHandle, PlatformDataProviderDelegate},
    error::NativeExtensionsResult,
    value_coerce::{CoerceToData, StringFormat},
};

use super::common::{atom_from_string, atom_to_string, TYPE_TEXT, TYPE_URI};

pub fn platform_stream_write(_handle: i32, _data: &[u8]) -> i32 {
    0
}

pub fn platform_stream_close(_handle: i32, _delete: bool) {}

pub struct PlatformDataProvider {
    weak_self: Late<Weak<Self>>,
    delegate: Weak<dyn PlatformDataProviderDelegate>,
    isolate_id: IsolateId,
    data: DataProvider,
}

impl PlatformDataProvider {
    pub fn new(
        delegate: Weak<dyn PlatformDataProviderDelegate>,
        isolate_id: IsolateId,
        data_provider: DataProvider,
    ) -> Self {
        Self {
            delegate,
            data: data_provider,
            isolate_id,
            weak_self: Late::new(),
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    pub async fn write_to_clipboard(
        providers: Vec<(Rc<PlatformDataProvider>, Arc<DataProviderHandle>)>,
    ) -> NativeExtensionsResult<()> {
        let data_object = DataObject::new(providers);
        data_object.write_to_clipboard().await
    }
}

struct ProviderEntry {
    provider: Rc<PlatformDataProvider>,
    _handle: Arc<DataProviderHandle>,
}

struct DataObject {
    providers: Vec<ProviderEntry>,
}

impl DataObject {
    pub fn new(providers: Vec<(Rc<PlatformDataProvider>, Arc<DataProviderHandle>)>) -> Rc<Self> {
        let res = Rc::new(Self {
            providers: providers
                .into_iter()
                .map(|p| ProviderEntry {
                    provider: p.0,
                    _handle: p.1,
                })
                .collect(),
        });
        res
    }

    fn set_data(selection_data: *mut GtkSelectionData, data: &[u8]) {
        unsafe {
            let mut target = gtk_selection_data_get_target(selection_data);
            let is_text = gtk_targets_include_text(&mut target as *mut _, 1) != GFALSE;
            if is_text {
                gtk_selection_data_set_text(
                    selection_data,
                    data.as_ptr() as *const _,
                    data.len() as c_int,
                );
            } else {
                gtk_selection_data_set(
                    selection_data,
                    target,
                    8,
                    data.as_ptr(),
                    data.len() as c_int,
                );
            }
        }
    }

    fn get_data_for_item(&self, item: &PlatformDataProvider, ty: &str) -> Option<Vec<u8>> {
        for data in &item.data.representations {
            match data {
                DataRepresentation::Simple { format, data } => {
                    if format == ty {
                        return data.coerce_to_data(StringFormat::Utf8);
                    }
                }
                DataRepresentation::Lazy { format, id } => {
                    if format == ty {
                        if let Some(delegate) = item.delegate.upgrade() {
                            let promise = delegate.get_lazy_data(item.isolate_id, *id, None);
                            loop {
                                if let Some(result) = promise.try_take() {
                                    match result {
                                        crate::value_promise::ValuePromiseResult::Ok { value } => {
                                            return value.coerce_to_data(StringFormat::Utf8);
                                        }
                                        crate::value_promise::ValuePromiseResult::Cancelled => {
                                            return None;
                                        }
                                    }
                                }
                                unsafe {
                                    gtk_main_iteration();
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn get_data(
        &self,
        _clipboard: *mut GtkClipboard,
        selection_data: *mut GtkSelectionData,
        _info: c_uint,
    ) {
        let mut target = unsafe { gtk_selection_data_get_target(selection_data) };
        let is_text = unsafe { gtk_targets_include_text(&mut target as *mut _, 1) } != GFALSE;
        let target = if is_text {
            TYPE_TEXT.to_owned()
        } else {
            unsafe { atom_to_string(&target) }
        };
        if target == TYPE_URI {
            // merge URIs from all items
            let mut data = Vec::<u8>::new();
            for item in &self.providers {
                if let Some(item_data) = self.get_data_for_item(&item.provider, &target) {
                    data.extend_from_slice(&item_data);
                    data.push(b'\r');
                    data.push(b'\n');
                }
            }
            Self::set_data(selection_data, &data);
        } else if let Some(item) = self.providers.first() {
            if let Some(data) = self.get_data_for_item(&item.provider, &target) {
                Self::set_data(selection_data, &data);
            }
        }
    }

    unsafe extern "C" fn _get_data(
        clipboard: *mut GtkClipboard,
        selection_data: *mut GtkSelectionData,
        info: c_uint,
        user_data: gpointer,
    ) {
        let user_data = user_data as *const DataObject;
        let this = ManuallyDrop::new(Rc::from_raw(user_data));
        this.get_data(clipboard, selection_data, info);
    }

    unsafe extern "C" fn _clear(_clipboard: *mut GtkClipboard, user_data: gpointer) {
        // Dealoc WeakPr
        let user_data = user_data as *const DataObject;
        Rc::from_raw(user_data);
    }

    pub async fn write_to_clipboard(self: &Rc<Self>) -> NativeExtensionsResult<()> {
        unsafe {
            let target_list = self.create_target_list();
            defer! { gtk_target_list_unref(target_list); }

            let mut n_targets: c_int = 0;
            let targets = gtk_target_table_new_from_list(target_list, &mut n_targets as *mut _);
            defer! { gtk_target_table_free(targets, n_targets); }

            let display = gdk_display_get_default();
            let clipboard = gtk_clipboard_get_default(display);

            let user_data = Rc::into_raw(self.clone());

            gtk_clipboard_set_with_data(
                clipboard,
                targets,
                n_targets as c_uint,
                Some(Self::_get_data),
                Some(Self::_clear),
                user_data as *mut _,
            );
        }

        Ok(())
    }

    fn add_types(target_list: *mut GtkTargetList, ty: &str) {
        if ty == TYPE_TEXT {
            unsafe { gtk_target_list_add_text_targets(target_list, 0) };
        } else {
            unsafe {
                gtk_target_list_add(target_list, atom_from_string(ty), 0, 0);
            };
        }
    }

    fn create_target_list(&self) -> *mut GtkTargetList {
        unsafe {
            let list = gtk_target_list_new(null_mut(), 0);
            if let Some(item) = self.providers.first() {
                for repr in &item.provider.data.representations {
                    match repr {
                        DataRepresentation::Simple { format, data: _ } => {
                            Self::add_types(list, format);
                        }
                        DataRepresentation::Lazy { format, id: _ } => {
                            Self::add_types(list, format);
                        }
                        _ => {}
                    }
                }
            }
            list
        }
    }
}
