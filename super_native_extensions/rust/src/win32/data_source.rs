use std::{
    cell::Cell,
    collections::HashMap,
    rc::Weak,
    sync::{Arc, Mutex},
};

use nativeshell_core::{util::Late, IsolateId};
use once_cell::sync::Lazy;
use windows::Win32::System::Ole::OleSetClipboard;

use crate::{
    api_model::DataSource, data_source_manager::PlatformDataSourceDelegate,
    error::NativeExtensionsResult, segmented_queue::SegmentedQueueWriter, util::DropNotifier,
};

use super::data_object::DataObject;

static STREAM_ENTRIES: Lazy<Mutex<HashMap<i32, SegmentedQueueWriter>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub(super) fn add_stream_entry(writer: SegmentedQueueWriter) -> i32 {
    fn next_stream_entry_handle() -> i32 {
        thread_local! {
            static NEXT_STREAM_ENTRY_HANDLE : Cell<i32>  = Cell::new(0)
        }
        NEXT_STREAM_ENTRY_HANDLE.with(|handle| {
            let res = handle.get();
            handle.set(res + 1);
            res
        })
    }
    let handle = next_stream_entry_handle();
    let mut entries = STREAM_ENTRIES.lock().unwrap();
    entries.insert(handle, writer);
    handle
}

pub fn platform_stream_write(handle: i32, data: &[u8]) -> i32 {
    let mut entries = STREAM_ENTRIES.lock().unwrap();
    let entry = entries.get_mut(&handle);
    match entry {
        Some(entry) => {
            entry.write(data);
            1
        }
        None => 0,
    }
}

pub fn platform_stream_close(handle: i32, _delete: bool) {
    let mut entries = STREAM_ENTRIES.lock().unwrap();
    let entry = entries.remove(&handle);
    if let Some(entry) = entry {
        entry.close();
    }
}

pub struct PlatformDataSource {
    weak_self: Late<Weak<Self>>,
    pub(super) isolate_id: IsolateId,
    pub(super) delegate: Weak<dyn PlatformDataSourceDelegate>,
    pub(super) data: DataSource,
}

impl PlatformDataSource {
    pub fn new(
        delegate: Weak<dyn PlatformDataSourceDelegate>,
        isolate_id: IsolateId,
        data: DataSource,
    ) -> Self {
        Self {
            weak_self: Late::new(),
            isolate_id,
            delegate,
            data,
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    pub async fn write_to_clipboard(
        &self,
        drop_notifier: Arc<DropNotifier>,
    ) -> NativeExtensionsResult<()> {
        let data_object = DataObject::create(self.weak_self.upgrade().unwrap(), drop_notifier);
        unsafe {
            OleSetClipboard(data_object)?;
        }
        Ok(())
    }
}
