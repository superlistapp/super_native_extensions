use std::{cell::RefCell, collections::HashMap, rc::Weak};

use nativeshell_core::{
    util::{FutureCompleter, Late},
    IsolateId,
};

use crate::{
    error::NativeExtensionsResult,
    value_promise::ValuePromiseResult,
    writer_data::{DataSource, ClipboardWriterItemData},
    writer_manager::PlatformDataSourceDelegate,
};

thread_local! {
    pub static WRITERS: RefCell<Vec<Weak<PlatformClipboardWriter>>> = RefCell::new(Vec::new());
}

pub struct PlatformClipboardWriter {
    weak_self: Late<Weak<Self>>,
    pub isolate_id: IsolateId,
    data: DataSource,
    pub written_data: RefCell<Option<DataSource>>,
    delegate: Weak<dyn PlatformDataSourceDelegate>,
    pub lazy_data: RefCell<HashMap<i64, ValuePromiseResult>>,
}

impl PlatformClipboardWriter {
    pub fn new(
        delegate: Weak<dyn PlatformDataSourceDelegate>,
        isolate_id: IsolateId,
        data: DataSource,
    ) -> Self {
        Self {
            weak_self: Late::new(),
            isolate_id,
            data,
            delegate,
            written_data: RefCell::new(None),
            lazy_data: RefCell::new(HashMap::new()),
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self.clone());
        WRITERS.with(|f| {
            f.borrow_mut().push(weak_self);
        })
    }

    pub async fn write_to_clipboard(&self) -> NativeExtensionsResult<()> {
        self.written_data.replace(Some(self.data.clone()));
        Ok(())
    }

    pub async fn request_all_lazy_items(&self) {
        for item in self.data.items.iter() {
            for data in item.data.iter() {
                if let ClipboardWriterItemData::Lazy {
                    types: _,
                    id: data_id,
                } = data
                {
                    let delegate = self.delegate.upgrade().unwrap();
                    let data = delegate
                        .get_lazy_data_async(self.isolate_id, *data_id)
                        .await;
                    self.lazy_data.borrow_mut().insert(*data_id, data);
                }
            }
        }
    }

    pub async fn request_lazy_item(&self, data_id: i64) {
        let (future, completer) = FutureCompleter::<()>::new();
        let res = self.delegate.upgrade().unwrap().get_lazy_data(
            self.isolate_id,
            data_id,
            Some(Box::new(move || {
                completer.complete(());
            })),
        );
        assert!(res.try_take().is_none());
        future.await;
        self.lazy_data.borrow_mut().insert(data_id, res.wait());
    }
}

impl Drop for PlatformClipboardWriter {
    fn drop(&mut self) {
        WRITERS.with(|f| {
            f.borrow_mut()
                .retain(|a| a.as_ptr() != self.weak_self.as_ptr());
        });
    }
}
