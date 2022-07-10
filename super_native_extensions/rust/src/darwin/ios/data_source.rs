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

use block::{Block, ConcreteBlock, RcBlock};
use cocoa::{
    base::{id, nil, BOOL},
    foundation::{NSArray, NSInteger, NSProcessInfo, NSUInteger, NSURL},
};
use nativeshell_core::{
    util::{Capsule, Late},
    Context, IsolateId, RunLoopSender, Value,
};
use objc::{
    class, msg_send,
    rc::{autoreleasepool, StrongPtr},
    sel, sel_impl,
};
use once_cell::sync::Lazy;

use crate::{
    api_model::{DataSource, DataSourceItemRepresentation, DataSourceValueId, VirtualFileStorage},
    data_source_manager::{PlatformDataSourceDelegate, VirtualFileResult},
    error::NativeExtensionsResult,
    log::OkLog,
    util::DropNotifier,
    value_coerce::{CoerceToData, StringFormat},
    value_promise::ValuePromiseResult, platform_impl::platform::common::{to_nsstring, from_nsstring, to_nserror},
};

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

/// DataSource state that may be accessed from multiple threads
struct PlatformDataSourceState {
    source: DataSource,
    precached_values: HashMap<(DataSourceValueId, String), ValuePromiseResult>,
}

pub struct PlatformDataSource {
    weak_self: Late<Weak<Self>>,
    delegate: Weak<dyn PlatformDataSourceDelegate>,
    isolate_id: IsolateId,
    state: Arc<Mutex<PlatformDataSourceState>>,
}

impl PlatformDataSource {
    pub fn new(
        delegate: Weak<dyn PlatformDataSourceDelegate>,
        isolate_id: IsolateId,
        source: DataSource,
    ) -> Self {
        Self {
            delegate,
            isolate_id,
            weak_self: Late::new(),
            state: Arc::new(Mutex::new(PlatformDataSourceState {
                source,
                precached_values: HashMap::new(),
            })),
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    pub fn create_items(
        &self,
        drop_notifier: Option<Arc<DropNotifier>>,
        delegate: Weak<dyn DataSourceSessionDelegate>,
    ) -> Vec<id> {
        let mut items = Vec::<id>::new();
        let session = DataSourceSession::new(
            self.state.clone(),
            drop_notifier,
            self.weak_self.clone(),
            delegate,
        );

        let state = self.state.lock().unwrap();
        for (index, item) in state.source.items.iter().enumerate() {
            unsafe {
                let item_provider: id = msg_send![class!(NSItemProvider), new];
                let item_provider: id = msg_send![item_provider, autorelease];
                if let Some(name) = &item.suggested_name {
                    let name = to_nsstring(name);
                    let () = msg_send![item_provider, setSuggestedName:*name];
                }
                for representation in &item.representations {
                    let formats = match representation {
                        DataSourceItemRepresentation::Simple { formats, data: _ } => Some(formats),
                        DataSourceItemRepresentation::Lazy { formats, id: _ } => Some(formats),
                        _ => None,
                    };
                    if let Some(formats) = formats {
                        for format in formats {
                            let session_clone = session.clone();
                            let format_clone = format.clone();
                            register_data_representation(item_provider, &format, move |callback| {
                                session_clone.value_for_index(index, &format_clone, callback)
                            });
                        }
                    }
                    if let DataSourceItemRepresentation::VirtualFile {
                        id,
                        format,
                        storage_suggestion,
                    } = representation
                    {
                        let storage =
                            storage_suggestion.unwrap_or(VirtualFileStorage::TemporaryFile);
                        let session_clone = session.clone();
                        let id = *id;
                        match storage {
                            VirtualFileStorage::TemporaryFile => {
                                register_file_representation(
                                    item_provider,
                                    &format,
                                    true,
                                    move |callback| {
                                        session_clone.file_for_index(id, storage, callback)
                                    },
                                );
                            }
                            VirtualFileStorage::Memory => {
                                register_data_representation(
                                    item_provider,
                                    &format,
                                    move |callback| {
                                        let callback2 =
                                            Box::new(move |data: id, _: bool, error: id| {
                                                callback(data, error)
                                            });
                                        session_clone.file_for_index(id, storage, callback2)
                                    },
                                );
                            }
                        }
                    }
                }

                items.push(item_provider);
            }
        }
        items
    }

    pub async fn write_to_clipboard(
        &self,
        drop_notifier: Arc<DropNotifier>,
    ) -> NativeExtensionsResult<()> {
        // iOS general pasteboard is truly braindead. It eagerly fetches all items invoking the
        // provider callbacks on background thread, but it blocks main thread on access until
        // the blocks return value. Which means if we try to schedule anything on main thread
        // it will deadlock. Because iOS prefetches everything anyway, we might as well do it
        // ourselves to avoid the deadlock.
        self.precache().await;
        autoreleasepool(|| unsafe {
            let items = self.create_items(Some(drop_notifier), self.weak_self.clone());
            let array = NSArray::arrayWithObjects(nil, &items);
            let pasteboard: id = msg_send![class!(UIPasteboard), generalPasteboard];
            let () = msg_send![pasteboard, setItemProviders: array];
        });
        Ok(())
    }

    async fn precache(&self) {
        let to_fetch = {
            let state = self.state.lock().unwrap();
            let mut items = Vec::<(DataSourceValueId, String)>::new();
            for item in &state.source.items {
                for data in &item.representations {
                    match data {
                        DataSourceItemRepresentation::Lazy { formats, id } => {
                            for format in formats {
                                let key = (*id, format.clone());
                                if !state.precached_values.contains_key(&key) {
                                    items.push(key);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            items
        };

        if let Some(delegate) = self.delegate.upgrade() {
            for item in to_fetch {
                let res = delegate
                    .get_lazy_data_async(self.isolate_id, item.0, item.1.clone())
                    .await;
                let mut state = self.state.lock().unwrap();
                state.precached_values.insert(item, res);
            }
        }
    }
}

impl DataSourceSessionDelegate for PlatformDataSource {
    fn should_fetch_items(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct Movable<T>(T);

unsafe impl<T> Send for Movable<T> {}

fn register_data_representation<F>(item_provider: id, type_identifier: &str, handler: F)
where
    F: Fn(Box<dyn Fn(id /* NSData */, id /* NSError */) + 'static + Send>) -> id + 'static + Send,
{
    let handler = Box::new(handler);
    let block = ConcreteBlock::new(move |completion_block: id| -> id {
        let completion_block = unsafe { &mut *(completion_block as *mut Block<(id, id), ()>) };
        let completion_block = unsafe { RcBlock::copy(completion_block) };
        let completion_block = Movable(completion_block);
        let completion_fn = move |data: id, err: id| {
            let completion_block = completion_block.clone();
            unsafe { completion_block.0.call((data, err)) };
        };
        handler(Box::new(completion_fn))
    });
    let block = block.copy();
    let type_identifier = to_nsstring(type_identifier);
    unsafe {
        let () = msg_send![item_provider,
            registerDataRepresentationForTypeIdentifier:*type_identifier
            visibility: 0 as NSUInteger // all
            loadHandler: &*block];
    }
}

fn register_file_representation<F>(
    item_provider: id,
    type_identifier: &str,
    open_in_place: bool,
    handler: F,
) where
    F: Fn(
            Box<dyn Fn(id /* NSURL */, bool /* coordinated */, id /* NSError */) + 'static + Send>,
        ) -> id /* NSProgress */
        + 'static
        + Send,
{
    let handler = Box::new(handler);
    let block = ConcreteBlock::new(move |completion_block: id| -> id {
        let completion_block =
            unsafe { &mut *(completion_block as *mut Block<(id, BOOL, id), ()>) };
        let completion_block = unsafe { RcBlock::copy(completion_block) };
        let completion_block = Movable(completion_block);
        let completion_fn = move |data: id, coordinated: bool, err: id| {
            let completion_block = completion_block.clone();
            unsafe { completion_block.0.call((data, coordinated as BOOL, err)) };
        };
        handler(Box::new(completion_fn))
    });
    let block = block.copy();
    let type_identifier = to_nsstring(type_identifier);
    unsafe {
        let () = msg_send![item_provider,
            registerFileRepresentationForTypeIdentifier:*type_identifier
            fileOptions: if open_in_place { 1 } else { 0 } as NSInteger
            visibility: 0 as NSUInteger // all
            loadHandler: &*block
        ];
    }
}

fn value_to_nsdata(value: &Value) -> StrongPtr {
    let buf = value.coerce_to_data(StringFormat::Utf8);
    match buf {
        Some(data) => to_nsdata(&data),
        None => unsafe { StrongPtr::new(std::ptr::null_mut()) },
    }
}

fn value_promise_res_to_nsdata(value: &ValuePromiseResult) -> StrongPtr {
    match value {
        ValuePromiseResult::Ok { value } => value_to_nsdata(value),
        ValuePromiseResult::Cancelled => unsafe { StrongPtr::new(std::ptr::null_mut()) },
    }
}

pub fn to_nsdata(data: &[u8]) -> StrongPtr {
    unsafe {
        let d: id = msg_send![class!(NSData), alloc];
        let d: id = msg_send![d, initWithBytes:data.as_ptr() length:data.len()];
        StrongPtr::new(d)
    }
}

pub trait DataSourceSessionDelegate {
    fn should_fetch_items(&self) -> bool;
}

// Make sure that DataSourceSession only has weak refernces to
// DataSource and DataSourceState. It may not get released because of iOS
// drag&drop memory leak where the item provider never gets released.
pub struct DataSourceSession {
    state: std::sync::Weak<Mutex<PlatformDataSourceState>>,
    _drop_notifier: Option<Arc<DropNotifier>>,
    sender: RunLoopSender,
    platform_source: Mutex<Capsule<Weak<PlatformDataSource>>>,
    delegate: Mutex<Capsule<Weak<dyn DataSourceSessionDelegate>>>,
    virtual_files: Mutex<Vec<Arc<DropNotifier>>>,
}

impl DataSourceSession {
    fn new(
        state: Arc<Mutex<PlatformDataSourceState>>,
        drop_notifier: Option<Arc<DropNotifier>>,
        platform_source: Weak<PlatformDataSource>,
        delegate: Weak<dyn DataSourceSessionDelegate>,
    ) -> Arc<Self> {
        let sender = Context::get().run_loop().new_sender();
        Arc::new(Self {
            state: Arc::downgrade(&state),
            _drop_notifier: drop_notifier,
            sender: sender.clone(),
            platform_source: Mutex::new(Capsule::new_with_sender(platform_source, sender.clone())),
            delegate: Mutex::new(Capsule::new_with_sender(delegate, sender)),
            virtual_files: Mutex::new(Vec::new()),
        })
    }

    fn on_platform_thread<F>(&self, f: F)
    where
        F: FnOnce(
                Option<(
                    Rc<PlatformDataSource>,
                    Rc<dyn PlatformDataSourceDelegate>,
                    Rc<dyn DataSourceSessionDelegate>,
                )>,
            )
            + 'static
            + Send,
    {
        let platform_source = self.platform_source.lock().unwrap().clone();
        let session_delegate = self.delegate.lock().unwrap().clone();
        self.sender.send(move || {
            // TODO(knopp) Simplify this if let_chain gets stabilized
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
        id: DataSourceValueId,
        format: String,
        callback: Box<dyn Fn(id, id) + Send>,
    ) -> id {
        Self::on_platform_thread(&self, move |s| match s {
            Some((source, source_delegate, session_delegate)) => {
                // For some reason iOS seems to eagerly fetch items immediatelly
                // at the beginning of drag (before even dragInteraction:sessionWillBegin:).
                // If we detect that return empty data.
                if !session_delegate.should_fetch_items() {
                    callback(nil, nil);
                    return;
                }
                Context::get().run_loop().spawn(async move {
                    let data = source_delegate
                        .get_lazy_data_async(source.isolate_id, id, format)
                        .await;
                    callback(*value_promise_res_to_nsdata(&data), nil);
                });
            }
            None => {
                callback(nil, nil);
            }
        });
        nil // NSProgress
    }

    fn value_for_index(
        self: &Arc<Self>,
        index: usize,
        format: &String,
        callback: Box<dyn Fn(id, id) + Send>,
    ) -> id {
        let state = match self.state.upgrade() {
            Some(state) => state,
            None => {
                callback(nil, nil);
                return nil;
            }
        };
        let state = state.lock().unwrap();
        let item = &state.source.items[index];
        for representation in &item.representations {
            match representation {
                DataSourceItemRepresentation::Simple { formats, data } => {
                    if formats.contains(format) {
                        let data = value_to_nsdata(data);
                        callback(*data, nil);
                        return nil;
                    }
                }
                DataSourceItemRepresentation::Lazy { formats, id } => {
                    if formats.contains(format) {
                        let precached = state.precached_values.get(&(*id, format.clone()));
                        match precached {
                            Some(value) => {
                                let data = value_promise_res_to_nsdata(value);
                                callback(*data, nil);
                                return nil;
                            }
                            None => return self.fetch_value(*id, format.clone(), callback),
                        }
                    }
                }
                _ => {}
            }
        }
        callback(nil, nil);
        nil // NSProgress
    }

    fn temp_file_path() -> PathBuf {
        let guid = unsafe {
            let info = NSProcessInfo::processInfo(nil);
            let string: id = msg_send![info, globallyUniqueString];
            from_nsstring(string)
        };
        temp_dir().join(guid)
    }

    fn new_stream_handle_for_storage(storage: VirtualFileStorage) -> Option<i32> {
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

    fn finish_stream_handle(stream_handle: i32) -> id {
        let stream_entry = STREAM_ENTRIES.lock().unwrap().remove(&stream_handle);
        match stream_entry {
            Some(StreamEntry::File { path, file }) => {
                drop(file);
                let path = path.to_string_lossy();
                let url = unsafe {
                    let url = NSURL::fileURLWithPath_(nil, *to_nsstring(&path));
                    let () = msg_send![class!(SNEDeletingPresenter), deleteAfterRead: url];
                    url
                };
                url
            }
            Some(StreamEntry::Memory { buffer }) => to_nsdata(&buffer).autorelease(),
            None => nil,
        }
    }

    fn fetch_virtual_file(
        self: &Arc<Self>,
        id: DataSourceValueId,
        progress: StrongPtr,
        storage: VirtualFileStorage,
        callback: Box<dyn Fn(id, bool, id) + Send>,
    ) {
        let progress = Movable(progress);
        let self_clone = self.clone();
        Self::on_platform_thread(&self, move |s| match s {
            Some((source, source_delegate, session_delegate)) => {
                let progress = progress;
                let progress = progress.0;
                // For some reason iOS seems to eagerly fetch items immediatelly
                // at the beginning of drag (before even dragInteraction:sessionWillBegin:).
                // If we detect that return empty data.
                if !session_delegate.should_fetch_items() {
                    callback(nil, false, nil);
                    return;
                }

                let stream_handle = Self::new_stream_handle_for_storage(storage);
                if stream_handle.is_none() {
                    callback(
                        nil,
                        false,
                        *to_nserror("super_dnd", 0, "Failed to open temporary file for writing"),
                    );
                    return;
                }
                let stream_handle = stream_handle.unwrap();

                let progress_clone = progress.clone();
                let notifier = source_delegate.get_virtual_file(
                    source.isolate_id,
                    id,
                    stream_handle,
                    Box::new(move |_| {}),
                    Box::new(move |cnt| {
                        let () = unsafe {
                            msg_send![*progress_clone, setCompletedUnitCount: cnt as u64]
                        };
                    }),
                    Box::new(move |result| match result {
                        VirtualFileResult::Done => {
                            let data = Self::finish_stream_handle(stream_handle);
                            callback(data, true /* must use presenter */, nil);
                        }
                        VirtualFileResult::Error { message } => {
                            let error = to_nserror("super_dnd", 0, &message);
                            callback(nil, false, *error);
                        }
                        VirtualFileResult::Cancelled => {
                            callback(nil, false, nil);
                        }
                    }),
                );
                self_clone
                    .virtual_files
                    .lock()
                    .unwrap()
                    .push(notifier.clone());
                let notifier = Arc::downgrade(&notifier);
                let cancellation_handler = ConcreteBlock::new(move || {
                    if let Some(notifier) = notifier.upgrade() {
                        notifier.dispose();
                    }
                });
                let cancellation_handler = cancellation_handler.copy();
                unsafe {
                    let () = msg_send![*progress, setCancellationHandler:&*cancellation_handler];
                }
            }
            None => {
                callback(nil, false, nil);
            }
        });
    }

    fn file_for_index(
        self: &Arc<Self>,
        id: DataSourceValueId,
        storage: VirtualFileStorage,
        callback: Box<dyn Fn(id, bool, id) + Send>,
    ) -> id {
        unsafe {
            let progress = StrongPtr::retain(
                msg_send![class!(NSProgress), progressWithTotalUnitCount: 100 as u64],
            );
            self.fetch_virtual_file(id, progress.clone(), storage, callback);
            *progress
        }
    }
}
