use std::{
    cell::Cell,
    collections::HashMap,
    env::temp_dir,
    fs::File,
    io::Write,
    path::PathBuf,
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
};

use block2::RcBlock;
use irondash_message_channel::{IsolateId, Late};
use irondash_run_loop::{spawn, util::Capsule, RunLoop, RunLoopSender};
use objc2::{
    extern_class, extern_methods, mutability::InteriorMutable, rc::Id, runtime::NSObject, ClassType,
};
use objc2_foundation::{
    NSArray, NSData, NSError, NSItemProvider, NSProcessInfo, NSProgress, NSString, NSURL,
};
use objc2_ui_kit::UIPasteboard;
use once_cell::sync::Lazy;

use crate::{
    api_model::{DataProvider, DataProviderValueId, DataRepresentation, VirtualFileStorage},
    data_provider_manager::{
        DataProviderHandle, PlatformDataProviderDelegate, VirtualFileResult, VirtualSessionHandle,
    },
    error::NativeExtensionsResult,
    log::OkLog,
    platform_impl::platform::common::to_nserror,
    util::Movable,
    value_promise::ValuePromiseResult,
};

use super::util::{
    register_data_representation, register_file_representation, value_promise_res_to_nsdata,
    value_to_nsdata,
};

/// DataSource state that may be accessed from multiple threads
struct PlatformDataProviderState {
    provider: DataProvider,
    precached_values: HashMap<DataProviderValueId, ValuePromiseResult>,
}

pub struct PlatformDataProvider {
    weak_self: Late<Weak<Self>>,
    delegate: Weak<dyn PlatformDataProviderDelegate>,
    isolate_id: IsolateId,
    state: Arc<Mutex<PlatformDataProviderState>>,
}

enum VirtualFilePayload {
    Url(Id<NSURL>),
    Data(Id<NSData>),
}

impl PlatformDataProvider {
    pub fn new(
        delegate: Weak<dyn PlatformDataProviderDelegate>,
        isolate_id: IsolateId,
        provider: DataProvider,
    ) -> Self {
        Self {
            delegate,
            isolate_id,
            weak_self: Late::new(),
            state: Arc::new(Mutex::new(PlatformDataProviderState {
                provider,
                precached_values: HashMap::new(),
            })),
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    pub fn create_ns_item_provider(
        &self,
        handle: Option<Arc<DataProviderHandle>>,
        delegate: Option<Weak<dyn DataProviderSessionDelegate>>,
    ) -> Id<NSItemProvider> {
        let delegate = delegate.unwrap_or_else(|| self.weak_self.clone());
        let session =
            DataProviderSession::new(self.state.clone(), handle, self.weak_self.clone(), delegate);
        let state = self.state.lock().unwrap();
        let item = &state.provider;
        unsafe {
            let item_provider = NSItemProvider::init(NSItemProvider::alloc());

            if let Some(name) = &item.suggested_name {
                item_provider.setSuggestedName(Some(&NSString::from_str(name)));
            }
            for representation in &item.representations {
                let format = match representation {
                    DataRepresentation::Simple { format, data: _ } => Some(format),
                    DataRepresentation::Lazy { format, id: _ } => Some(format),
                    _ => None,
                };
                if let Some(format) = format {
                    let session_clone = session.clone();
                    let format_clone = format.clone();
                    register_data_representation(&item_provider, format, move |callback| {
                        session_clone.value_for_format(&format_clone, callback)
                    });
                }
                if let DataRepresentation::VirtualFile {
                    id,
                    format,
                    storage_suggestion,
                } = representation
                {
                    let storage = storage_suggestion.unwrap_or(VirtualFileStorage::TemporaryFile);
                    let session_clone = session.clone();
                    let id = *id;
                    match storage {
                        VirtualFileStorage::TemporaryFile => {
                            register_file_representation(
                                &item_provider,
                                format,
                                true,
                                move |callback| {
                                    let callback2 = Box::new(
                                        move |data: Option<VirtualFilePayload>,
                                              must_use_presenter: bool,
                                              error: Option<&NSError>| {
                                            match data {
                                                Some(VirtualFilePayload::Url(url)) => {
                                                    callback(Some(&url), must_use_presenter, error)
                                                },
                                                None => {
                                                    callback(None, must_use_presenter, error)
                                                },
                                                _ => {
                                                    panic!("Unexpected data type");
                                                }
                                            }
                                        },
                                    );
                                    session_clone.file_representation(id, storage, callback2)
                                },
                            );
                        }
                        VirtualFileStorage::Memory => {
                            register_data_representation(&item_provider, format, move |callback| {
                                let callback2 = Box::new(move |data: Option<VirtualFilePayload>, _: bool, error: Option<&NSError>| {
                                    match data {
                                        Some(VirtualFilePayload::Data(data)) => {
                                            callback(Some(&data), error)
                                        },
                                        None => {
                                            callback(None,  error)
                                        },
                                        _ => {
                                            panic!("Unexpected data type");
                                        }
                                    };
                                });
                                session_clone.file_representation(id, storage, callback2)
                            });
                        }
                    }
                }
            }
            item_provider
        }
    }

    pub async fn write_to_clipboard(
        providers: Vec<(Rc<PlatformDataProvider>, Arc<DataProviderHandle>)>,
    ) -> NativeExtensionsResult<()> {
        for provider in &providers {
            provider.0.precache().await;
        }

        let providers: Vec<Id<NSItemProvider>> = providers
            .into_iter()
            .map(|(platform_data_provider, provider_handle)| {
                platform_data_provider.create_ns_item_provider(Some(provider_handle), None)
            })
            .collect();

        let array = NSArray::from_vec(providers);
        let pasteboard = unsafe { UIPasteboard::generalPasteboard() };
        unsafe { pasteboard.setItemProviders(&array) };

        Ok(())
    }

    async fn precache(&self) {
        let to_fetch = {
            let state = self.state.lock().unwrap();
            let mut items = Vec::<DataProviderValueId>::new();
            for data in &state.provider.representations {
                match data {
                    DataRepresentation::Lazy { format: _, id } => {
                        if !state.precached_values.contains_key(id) {
                            items.push(*id);
                        }
                    }
                    _ => {}
                }
            }

            items
        };

        if let Some(delegate) = self.delegate.upgrade() {
            for item in to_fetch {
                let res = delegate.get_lazy_data_async(self.isolate_id, item).await;
                let mut state = self.state.lock().unwrap();
                state.precached_values.insert(item, res);
            }
        }
    }
}

pub trait DataProviderSessionDelegate {
    fn should_fetch_items(&self) -> bool;
}

// Make sure that DataSourceSession only has weak weak to
// DataSource and DataSourceState. It may not get released because of iOS
// drag and drop memory leak where the item provider never gets released.
pub struct DataProviderSession {
    state: std::sync::Weak<Mutex<PlatformDataProviderState>>,
    _provider_handle: Option<Arc<DataProviderHandle>>,
    sender: RunLoopSender,
    platform_provider: Mutex<Capsule<Weak<PlatformDataProvider>>>,
    delegate: Mutex<Capsule<Weak<dyn DataProviderSessionDelegate>>>,
    virtual_files: Mutex<Vec<Arc<VirtualSessionHandle>>>,
}

impl DataProviderSession {
    fn new(
        state: Arc<Mutex<PlatformDataProviderState>>,
        provider_handle: Option<Arc<DataProviderHandle>>,
        platform_provider: Weak<PlatformDataProvider>,
        delegate: Weak<dyn DataProviderSessionDelegate>,
    ) -> Arc<Self> {
        let sender = RunLoop::current().new_sender();
        Arc::new(Self {
            state: Arc::downgrade(&state),
            _provider_handle: provider_handle,
            sender: sender.clone(),
            platform_provider: Mutex::new(Capsule::new_with_sender(
                platform_provider,
                sender.clone(),
            )),
            delegate: Mutex::new(Capsule::new_with_sender(delegate, sender)),
            virtual_files: Mutex::new(Vec::new()),
        })
    }

    fn on_platform_thread<F>(&self, f: F)
    where
        F: FnOnce(
                Option<(
                    Rc<PlatformDataProvider>,
                    Rc<dyn PlatformDataProviderDelegate>,
                    Rc<dyn DataProviderSessionDelegate>,
                )>,
            )
            + 'static
            + Send,
    {
        let platform_source = self.platform_provider.lock().unwrap().clone();
        let session_delegate = self.delegate.lock().unwrap().clone();
        self.sender.send(move || {
            // TODO(knopp): Simplify this if let_chain gets stabilized
            if let (Some(session_delegate), Some((source, source_delegate))) = (
                session_delegate.get_ref().ok().and_then(|s| s.upgrade()),
                platform_source
                    .get_ref()
                    .ok()
                    .and_then(|c| c.upgrade())
                    .and_then(|s| s.delegate.upgrade().map(|delegate| (s, delegate))),
            ) {
                f(Some((source, source_delegate, session_delegate)));
            } else {
                f(None)
            }
        });
    }

    fn fetch_value(
        &self,
        id: DataProviderValueId,
        callback: Box<dyn Fn(Option<&NSData>, Option<&NSError>) + Send>,
    ) -> Option<Id<NSProgress>> {
        Self::on_platform_thread(self, move |s| match s {
            Some((source, source_delegate, session_delegate)) => {
                // For some reason iOS seems to eagerly fetch items immediately
                // at the beginning of drag (before even dragInteraction:sessionWillBegin:).
                // If we detect that return empty data.
                if !session_delegate.should_fetch_items() {
                    callback(None, None);
                    return;
                }
                spawn(async move {
                    let data = source_delegate
                        .get_lazy_data_async(source.isolate_id, id)
                        .await;
                    let data = value_promise_res_to_nsdata(&data);
                    callback(data.as_deref(), None);
                });
            }
            None => {
                callback(None, None);
            }
        });
        None
    }

    fn value_for_format(
        self: &Arc<Self>,
        requested_format: &String,
        callback: Box<dyn Fn(Option<&NSData>, Option<&NSError>) + Send>,
    ) -> Option<Id<NSProgress>> {
        let state = match self.state.upgrade() {
            Some(state) => state,
            None => {
                callback(None, None);
                return None;
            }
        };
        let state = state.lock().unwrap();
        let provider = &state.provider;
        for representation in &provider.representations {
            match representation {
                DataRepresentation::Simple { format, data } => {
                    if format == requested_format {
                        let data = value_to_nsdata(data);
                        callback(data.as_deref(), None);
                        return None;
                    }
                }
                DataRepresentation::Lazy { format, id } => {
                    if requested_format == format {
                        let precached = state.precached_values.get(id);
                        match precached {
                            Some(value) => {
                                let data = value_promise_res_to_nsdata(value);
                                callback(data.as_deref(), None);
                                return None;
                            }
                            None => return self.fetch_value(*id, callback),
                        }
                    }
                }
                _ => {}
            }
        }
        callback(None, None);
        None
    }

    fn temp_file_path() -> PathBuf {
        let guid = unsafe {
            let info = NSProcessInfo::processInfo();
            let string = info.globallyUniqueString();
            string.to_string()
        };
        temp_dir().join(guid)
    }

    fn new_stream_handle_for_storage(storage: VirtualFileStorage) -> Option<i32> {
        fn next_stream_entry_handle() -> i32 {
            thread_local! {
                static NEXT_STREAM_ENTRY_HANDLE : Cell<i32> = const { Cell::new(0) }
            }
            NEXT_STREAM_ENTRY_HANDLE.with(|handle| {
                let res = handle.get();
                handle.set(res + 1);
                res
            })
        }
        match storage {
            VirtualFileStorage::TemporaryFile => {
                let path = Self::temp_file_path();
                let file = File::create(&path).ok_log()?;
                let handle = next_stream_entry_handle();
                STREAM_ENTRIES
                    .lock()
                    .unwrap()
                    .insert(handle, StreamEntry::File { path, file });
                Some(handle)
            }
            VirtualFileStorage::Memory => {
                let handle = next_stream_entry_handle();
                STREAM_ENTRIES
                    .lock()
                    .unwrap()
                    .insert(handle, StreamEntry::Memory { buffer: Vec::new() });
                Some(handle)
            }
        }
    }

    fn finish_stream_handle(stream_handle: i32) -> Option<VirtualFilePayload> {
        let stream_entry = STREAM_ENTRIES.lock().unwrap().remove(&stream_handle);
        match stream_entry {
            Some(StreamEntry::File { path, file }) => {
                drop(file);
                let path = path.to_string_lossy();
                let url = unsafe { NSURL::fileURLWithPath(&NSString::from_str(&path)) };
                unsafe { SNEDeletingPresenter::deleteAfterRead(&url) };
                Some(VirtualFilePayload::Url(url))
            }
            Some(StreamEntry::Memory { buffer }) => {
                Some(VirtualFilePayload::Data(NSData::from_vec(buffer)))
            }
            None => None,
        }
    }

    fn fetch_virtual_file(
        self: &Arc<Self>,
        id: DataProviderValueId,
        progress: &NSProgress,
        storage: VirtualFileStorage,
        callback: Box<dyn Fn(Option<VirtualFilePayload>, bool, Option<&NSError>) + Send>,
    ) {
        let progress = progress.retain();
        let progress = unsafe { Movable::new(progress) };
        let self_clone = self.clone();
        Self::on_platform_thread(self, move |s| match s {
            Some((source, source_delegate, session_delegate)) => {
                let progress = progress.take();
                // For some reason iOS seems to eagerly fetch items immediately
                // at the beginning of drag (before even dragInteraction:sessionWillBegin:).
                // If we detect that return empty data.
                if !session_delegate.should_fetch_items() {
                    callback(None, false, None);
                    return;
                }

                let stream_handle = Self::new_stream_handle_for_storage(storage);
                if stream_handle.is_none() {
                    callback(
                        None,
                        false,
                        Some(&to_nserror(
                            "super_dnd",
                            0,
                            "Failed to open temporary file for writing",
                        )),
                    );
                    return;
                }
                let stream_handle = stream_handle.unwrap();

                let progress_clone = progress.retain();
                let notifier = source_delegate.get_virtual_file(
                    source.isolate_id,
                    id,
                    stream_handle,
                    Box::new(move |_| {}),
                    Box::new(move |cnt| {
                        let completed = (cnt * 1000.0).round() as i64;
                        unsafe { progress_clone.setCompletedUnitCount(completed) };
                    }),
                    Box::new(move |result| match result {
                        VirtualFileResult::Done => {
                            let data = Self::finish_stream_handle(stream_handle);
                            callback(data, true /* must use presenter */, None);
                        }
                        VirtualFileResult::Error { message } => {
                            let error = to_nserror("super_dnd", 0, &message);
                            callback(None, false, Some(&error));
                        }
                        VirtualFileResult::Cancelled => {
                            callback(None, false, None);
                        }
                    }),
                );
                self_clone
                    .virtual_files
                    .lock()
                    .unwrap()
                    .push(notifier.clone());
                let notifier = Arc::downgrade(&notifier);
                let cancellation_handler = RcBlock::new(move || {
                    if let Some(notifier) = notifier.upgrade() {
                        notifier.dispose();
                    }
                });

                unsafe { progress.setCancellationHandler(Some(&cancellation_handler)) };
            }
            None => {
                callback(None, false, None);
            }
        });
    }

    fn file_representation(
        self: &Arc<Self>,
        id: DataProviderValueId,
        storage: VirtualFileStorage,
        callback: Box<dyn Fn(Option<VirtualFilePayload>, bool, Option<&NSError>) + Send>,
    ) -> Option<Id<NSProgress>> {
        let progress = unsafe { NSProgress::progressWithTotalUnitCount(1000) };
        self.fetch_virtual_file(id, &progress, storage, callback);
        Some(progress)
    }
}

impl DataProviderSessionDelegate for PlatformDataProvider {
    fn should_fetch_items(&self) -> bool {
        true
    }
}

//
// Virtual file streams
//

enum StreamEntry {
    File { path: PathBuf, file: File },
    Memory { buffer: Vec<u8> },
}

static STREAM_ENTRIES: Lazy<Mutex<HashMap<i32, StreamEntry>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn platform_stream_write(handle: i32, data: &[u8]) -> i32 {
    let mut entries = STREAM_ENTRIES.lock().unwrap();
    let entry = entries.get_mut(&handle);
    if let Some(mut entry) = entry {
        match &mut entry {
            StreamEntry::File { path: _, file } => match file.write_all(data) {
                Ok(_) => 1,
                Err(_) => 0,
            },
            StreamEntry::Memory { buffer } => {
                buffer.extend_from_slice(data);
                1
            }
        }
    } else {
        0
    }
}

pub fn platform_stream_close(handle: i32, delete: bool) {
    if delete {
        let entry = {
            let mut entries = STREAM_ENTRIES.lock().unwrap();
            entries.remove(&handle)
        };
        if let Some(entry) = entry {
            match entry {
                StreamEntry::File { path, file } => {
                    drop(file);
                    std::fs::remove_file(path).unwrap();
                }
                StreamEntry::Memory { buffer: _ } => {}
            }
        }
    }
}

extern_class!(
    #[derive(PartialEq, Eq, Hash)]
    pub struct SNEDeletingPresenter;

    unsafe impl ClassType for SNEDeletingPresenter {
        type Super = NSObject;
        type Mutability = InteriorMutable;
    }
);

extern_methods!(
    unsafe impl SNEDeletingPresenter {
        #[method(deleteAfterRead:)]
        #[allow(non_snake_case)]
        pub unsafe fn deleteAfterRead(url: &NSURL);
    }
);
