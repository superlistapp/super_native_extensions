use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
    sync::Arc,
};

use gdk::{Atom, Display};

use gtk::{Clipboard, SelectionData, TargetList};
use irondash_message_channel::{IsolateId, Late};
use irondash_run_loop::RunLoop;

use crate::{
    api_model::{DataProvider, DataProviderValueId, DataRepresentation},
    data_provider_manager::{DataProviderHandle, PlatformDataProviderDelegate},
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    value_coerce::{CoerceToData, StringFormat},
};

use super::common::{target_includes_text, TargetListExt, TYPE_TEXT, TYPE_URI};

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
        data_object.write_to_clipboard()
    }
}

struct ProviderEntry {
    provider: Rc<PlatformDataProvider>,
    _handle: Arc<DataProviderHandle>,
}

pub struct DataObject {
    providers: Vec<ProviderEntry>,
    cache: RefCell<HashMap<DataProviderValueId, Option<Vec<u8>>>>,
}

impl DataObject {
    pub fn new(providers: Vec<(Rc<PlatformDataProvider>, Arc<DataProviderHandle>)>) -> Rc<Self> {
        Rc::new(Self {
            providers: providers
                .into_iter()
                .map(|p| ProviderEntry {
                    provider: p.0,
                    _handle: p.1,
                })
                .collect(),
            cache: RefCell::new(HashMap::new()),
        })
    }

    fn set_data_(selection_data: &SelectionData, data: &[u8]) -> NativeExtensionsResult<()> {
        let target = selection_data.target();
        if target_includes_text(&target) {
            selection_data.set_text(
                std::str::from_utf8(data)
                    .map_err(|e| NativeExtensionsError::OtherError(e.to_string()))?,
            );
        } else {
            selection_data.set(&target, 8, data);
        }
        Ok(())
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
                        if let Some(cached) = self.cache.borrow().get(id).cloned() {
                            return cached;
                        }
                        if let Some(delegate) = item.delegate.upgrade() {
                            let promise = delegate.get_lazy_data(item.isolate_id, *id, None);
                            loop {
                                if let Some(result) = promise.try_take() {
                                    match result {
                                        crate::value_promise::ValuePromiseResult::Ok { value } => {
                                            let data = value.coerce_to_data(StringFormat::Utf8);
                                            self.cache.borrow_mut().insert(*id, data.clone());
                                            return data;
                                        }
                                        crate::value_promise::ValuePromiseResult::Cancelled => {
                                            return None;
                                        }
                                    }
                                }
                                RunLoop::current().platform_run_loop.poll_once();
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    pub fn get_data(&self, selection_data: &SelectionData) -> NativeExtensionsResult<()> {
        let target = selection_data.target();
        let is_text = target_includes_text(&target);

        let target = if is_text {
            TYPE_TEXT.to_owned()
        } else {
            target.name().as_str().to_owned()
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
            Self::set_data_(selection_data, &data)?;
        } else if let Some(item) = self.providers.first() {
            if let Some(data) = self.get_data_for_item(&item.provider, &target) {
                Self::set_data_(selection_data, &data)?;
            }
        }
        Ok(())
    }

    pub fn write_to_clipboard(self: &Rc<Self>) -> NativeExtensionsResult<()> {
        unsafe { gtk::set_initialized() };
        let list = self.create_target_list();
        let targets = list.get_target_entries();
        let display = Display::default()
            .ok_or_else(|| NativeExtensionsError::OtherError("Display not found".into()))?;
        let clipboard = Clipboard::default(&display)
            .ok_or_else(|| NativeExtensionsError::OtherError("Clipboard not found".into()))?;
        let self_clone = self.clone();
        clipboard.set_with_data(&targets, move |_, selection_data, _| {
            self_clone.get_data(selection_data).ok_log();
        });
        Ok(())
    }

    pub fn create_target_list(&self) -> TargetList {
        let list = TargetList::new(&[]);
        fn add(list: &TargetList, ty: &str) {
            if ty == TYPE_TEXT {
                list.add_text_targets(0);
            } else {
                list.add(&Atom::intern(ty), 0, 0);
            }
        }
        if let Some(item) = self.providers.first() {
            for repr in &item.provider.data.representations {
                match repr {
                    DataRepresentation::Simple { format, data: _ } => {
                        add(&list, format);
                    }
                    DataRepresentation::Lazy { format, id: _ } => {
                        add(&list, format);
                    }
                    _ => {}
                }
            }
        }
        list
    }
}
