use std::{
    cell::RefCell,
    fs::{self, File},
    io::Read,
    path::PathBuf,
    ptr::NonNull,
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
    thread,
};

use async_trait::async_trait;

use block2::RcBlock;
use irondash_message_channel::{value_darwin::ValueObjcConversion, Value};
use irondash_run_loop::{
    util::{Capsule, FutureCompleter},
    RunLoop,
};
use objc2_foundation::{
    NSArray, NSCopying, NSData, NSError, NSFileCoordinator, NSFileCoordinatorReadingOptions,
    NSItemProvider, NSPropertyListReadOptions, NSPropertyListSerialization, NSString, NSURL,
};

use objc2::{
    rc::{autoreleasepool, Id},
    runtime::{Bool, NSObject},
    ClassType,
};
use objc2_ui_kit::{UIDragItem, UIPasteboard};

use crate::{
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    platform_impl::platform::{
        common::{path_from_url, uti_conforms_to, NSURLSecurtyScopeAccess},
        progress_bridge::bridge_progress,
    },
    reader_manager::{ReadProgress, VirtualFileReader},
    util::{get_target_path, Movable},
    value_promise::Promise,
};

pub struct PlatformDataReader {
    source: ReaderSource,
}

enum ReaderSource {
    Pasteboard(Id<UIPasteboard>),
    DropSessionItems(Id<NSArray<UIDragItem>>),
}

impl PlatformDataReader {
    fn get_items_providers(&self) -> Vec<Id<NSItemProvider>> {
        match &self.source {
            ReaderSource::Pasteboard(pasteboard) => {
                let providers = unsafe { pasteboard.itemProviders() };
                providers.iter().map(|e| e.retain()).collect()
            }
            ReaderSource::DropSessionItems(items) => items
                .iter()
                .map(|item| unsafe { item.itemProvider() })
                .collect(),
        }
    }

    pub async fn get_items(&self) -> NativeExtensionsResult<Vec<i64>> {
        let count = {
            let providers = self.get_items_providers();
            providers.len() as i64
        };
        Ok((0..count).collect())
    }

    pub fn get_formats_for_item_sync(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        let formats = unsafe {
            let providers = self.get_items_providers();
            if item < providers.len() as i64 {
                let provider = &providers[item as usize];
                let identifiers = provider.registeredTypeIdentifiers();
                identifiers.iter().map(|e| e.to_string()).collect()
            } else {
                Vec::new()
            }
        };
        Ok(formats)
    }

    pub async fn get_formats_for_item(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        self.get_formats_for_item_sync(item)
    }

    pub async fn get_suggested_name_for_item(
        &self,
        item: i64,
    ) -> NativeExtensionsResult<Option<String>> {
        let name = unsafe {
            let providers = self.get_items_providers();
            if item < providers.len() as i64 {
                let provider = &providers[item as usize];
                let name = provider.suggestedName();
                name.map(|name| name.to_string())
            } else {
                None
            }
        };
        Ok(name)
    }

    unsafe fn maybe_decode_bplist(data: &NSData) -> Id<NSObject> {
        let data_slice = data.bytes();
        let magic: &[u8; 8] = &[98, 112, 108, 105, 115, 116, 48, 48];
        if data_slice.starts_with(magic) {
            let list = NSPropertyListSerialization::propertyListWithData_options_format_error(
                data,
                NSPropertyListReadOptions::NSPropertyListImmutable,
                std::ptr::null_mut(),
            );
            if let Ok(list) = list {
                Id::cast(list)
            } else {
                Id::cast(data.copy())
            }
        } else {
            Id::cast(data.copy())
        }
    }

    pub async fn get_data_for_item(
        &self,
        item: i64,
        format: String,
        read_progress: Option<Arc<ReadProgress>>,
    ) -> NativeExtensionsResult<Value> {
        let (future, completer) = FutureCompleter::new();
        unsafe {
            let providers = self.get_items_providers();
            if item < providers.len() as i64 {
                // travels between threads, must be refcounted because block is Fn
                let completer = Arc::new(Mutex::new(Capsule::new(completer)));
                let provider = &providers[item as usize];
                let sender = RunLoop::current().new_sender();
                let block = RcBlock::new(move |data: *mut NSData, _err: *mut NSError| {
                    let data = Id::retain(data);
                    let data = data.map(|d| Self::maybe_decode_bplist(&d));
                    let data = Movable::new(data);
                    let completer = completer.clone();
                    sender.send(move || {
                        let completer = completer
                            .lock()
                            .unwrap()
                            .take()
                            .expect("Block invoked more than once");
                        let data = data;
                        let data = data.take();
                        completer.complete(Ok(Value::from_objc(data).ok_log().unwrap_or_default()))
                    });
                });
                let ns_progress = provider
                    .loadDataRepresentationForTypeIdentifier_completionHandler(
                        &NSString::from_str(&format),
                        &block,
                    );

                if let Some(read_progress) = read_progress {
                    bridge_progress(ns_progress, read_progress);
                }
            } else {
                completer.complete(Ok(Value::Null));
            }
        };
        future.await
    }

    pub fn new_clipboard_reader() -> NativeExtensionsResult<Rc<Self>> {
        let res = Rc::new(Self {
            source: ReaderSource::Pasteboard(unsafe { UIPasteboard::generalPasteboard() }),
        });
        res.assign_weak_self(Rc::downgrade(&res));
        Ok(res)
    }

    pub fn new_with_drop_session_items(
        items: Id<NSArray<UIDragItem>>,
    ) -> NativeExtensionsResult<Rc<Self>> {
        let res = Rc::new(Self {
            source: ReaderSource::DropSessionItems(items),
        });
        res.assign_weak_self(Rc::downgrade(&res));
        Ok(res)
    }

    pub fn item_format_is_synthesized(
        &self,
        _item: i64,
        _format: &str,
    ) -> NativeExtensionsResult<bool> {
        Ok(false)
    }

    pub async fn can_copy_virtual_file_for_item(
        &self,
        item: i64,
        format: &str,
    ) -> NativeExtensionsResult<bool> {
        // All data on iOS can be received as a virtual file through
        // load(InPlace)FileRepresentationForTypeIdentifier so we add a bit of heuristics
        // to avoid creating a potential temporary file for text, composite content, images
        // and URLS. The assumption here is that they are small enough to be
        // to be all loaded in memory.
        if uti_conforms_to(format, "public.composite-content")
            || uti_conforms_to(format, "public.text")
            || uti_conforms_to(format, "public.image")
            || uti_conforms_to(format, "public.url")
            || uti_conforms_to(format, "com.apple.property-list")
        {
            return Ok(false);
        }
        let formats = self.get_formats_for_item_sync(item)?;
        Ok(formats.iter().any(|f| f == format))
    }

    pub async fn get_item_format_for_uri(
        &self,
        _item: i64,
    ) -> NativeExtensionsResult<Option<String>> {
        Ok(None)
    }

    pub async fn can_read_virtual_file_for_item(
        &self,
        item: i64,
        format: &str,
    ) -> NativeExtensionsResult<bool> {
        self.can_copy_virtual_file_for_item(item, format).await
    }

    pub async fn create_virtual_file_reader_for_item(
        &self,
        item: i64,
        format: &str,
        read_progress: Arc<ReadProgress>,
    ) -> NativeExtensionsResult<Option<Rc<dyn VirtualFileReader>>> {
        let providers = self.get_items_providers();
        if item >= providers.len() as i64 {
            return Err(NativeExtensionsError::OtherError("Invalid item".into()));
        }
        let (future, completer) = FutureCompleter::new();

        // travels between threads, must be refcounted because block is Fn
        let completer = Arc::new(Mutex::new(Capsule::new(completer)));
        let provider = &providers[item as usize];
        let sender = RunLoop::current().new_sender();
        let block = RcBlock::new(
            move |url: *mut NSURL, _is_in_place: Bool, error: *mut NSError| {
                let url = unsafe { Id::retain(url) };
                let error = unsafe { Id::retain(error) };
                let res = match (url, error) {
                    (Some(url), _) => FileWithBackgroundCoordinator::new(&url),
                    (_, Some(error)) => Err(NativeExtensionsError::VirtualFileReceiveError(
                        error.localizedDescription().to_string(),
                    )),
                    (_, _) => Err(NativeExtensionsError::VirtualFileReceiveError(
                        "Unknown error".into(),
                    )),
                };
                let completer = completer.clone();
                sender.send(move || {
                    let completer = completer
                        .lock()
                        .unwrap()
                        .take()
                        .expect("Block invoked more than once");
                    // completer.complete(res);
                    let res = res.map::<Option<Rc<dyn VirtualFileReader>>, _>(|f| Some(Rc::new(f)));
                    completer.complete(res);
                });
            },
        );
        let ns_progress = unsafe {
            provider.loadInPlaceFileRepresentationForTypeIdentifier_completionHandler(
                &NSString::from_str(format),
                &block,
            )
        };
        bridge_progress(ns_progress, read_progress);
        future.await
    }

    pub async fn copy_virtual_file_for_item(
        &self,
        item: i64,
        format: &str,
        target_folder: PathBuf,
        read_progress: Arc<ReadProgress>,
    ) -> NativeExtensionsResult<PathBuf> {
        let (future, completer) = FutureCompleter::new();
        let providers = self.get_items_providers();
        if item >= providers.len() as i64 {
            return Err(NativeExtensionsError::OtherError("Invalid item".into()));
        }

        // travels between threads, must be refcounted because block is Fn
        let completer = Arc::new(Mutex::new(Capsule::new(completer)));
        let provider = &providers[item as usize];
        let sender: irondash_run_loop::RunLoopSender = RunLoop::current().new_sender();
        let block = RcBlock::new(move |url: *mut NSURL, error: *mut NSError| {
            let url = unsafe { Id::retain(url) };
            let error = unsafe { Id::retain(error) };
            let res = match (url, error) {
                (Some(url), _) => {
                    let source_path = path_from_url(&url);
                    let source_name = source_path
                        .file_name()
                        .expect("Missing file name")
                        .to_string_lossy();
                    let target_path = get_target_path(&target_folder, &source_name);
                    match fs::rename(&source_path, &target_path) {
                        Ok(_) => Ok(target_path),
                        Err(err) => Err(NativeExtensionsError::VirtualFileReceiveError(
                            err.to_string(),
                        )),
                    }
                }
                (_, Some(error)) => Err(NativeExtensionsError::VirtualFileReceiveError(
                    error.localizedDescription().to_string(),
                )),
                (_, _) => Err(NativeExtensionsError::VirtualFileReceiveError(
                    "Unknown error".into(),
                )),
            };
            let completer = completer.clone();
            sender.send(move || {
                let completer = completer
                    .lock()
                    .unwrap()
                    .take()
                    .expect("Block invoked more than once");
                completer.complete(res);
            });
        });
        let ns_progress = unsafe {
            provider.loadFileRepresentationForTypeIdentifier_completionHandler(
                &NSString::from_str(format),
                &block,
            )
        };
        bridge_progress(ns_progress, read_progress);

        future.await
    }

    pub fn assign_weak_self(&self, _weak: Weak<PlatformDataReader>) {}
}

struct FileWithBackgroundCoordinator {
    // used to block the coordinator thread
    coordinator_thread_release: Arc<Promise<()>>,
    file: RefCell<Option<File>>,
    path: PathBuf,
}

impl FileWithBackgroundCoordinator {
    /// Creates new thread where it keeps the coordinator alive while the
    /// FileWithBackgroundCoordinator is reading file.
    fn new(url: &NSURL) -> NativeExtensionsResult<FileWithBackgroundCoordinator> {
        let url = url.retain();
        let promise = Arc::new(Promise::new());
        let url = unsafe { Movable::new(url) };
        let promise_clone = promise.clone();
        let promise_clone2 = promise.clone();
        thread::spawn(move || {
            let block = RcBlock::new(move |new_url: NonNull<NSURL>| {
                let new_url = unsafe { Id::retain(new_url.as_ptr()).unwrap() };
                let _access = NSURLSecurtyScopeAccess::new(&new_url);
                let path = path_from_url(&new_url);
                let release = Arc::new(Promise::new());
                let file = File::open(&path);
                match file {
                    Ok(file) => {
                        promise_clone2.set(Ok(FileWithBackgroundCoordinator {
                            coordinator_thread_release: release.clone(),
                            file: RefCell::new(Some(file)),
                            path,
                        }));
                        // wait until the file is closed
                        release.wait();
                    }
                    Err(err) => {
                        promise_clone2.set(Err(NativeExtensionsError::from(err)));
                    }
                }
            });
            let mut error: Option<Id<NSError>> = None;
            autoreleasepool(|_| unsafe {
                url.startAccessingSecurityScopedResource();
                let coordinator =
                    NSFileCoordinator::initWithFilePresenter(NSFileCoordinator::alloc(), None);
                coordinator.coordinateReadingItemAtURL_options_error_byAccessor(
                    &url,
                    NSFileCoordinatorReadingOptions::NSFileCoordinatorReadingWithoutChanges,
                    Some(&mut error),
                    &block,
                );
                if let Some(error) = error {
                    promise_clone.set(Err(NativeExtensionsError::VirtualFileReceiveError(
                        error.localizedDescription().to_string(),
                    )));
                }
                url.stopAccessingSecurityScopedResource();
            });
        });
        promise.wait()
    }
}

impl Drop for FileWithBackgroundCoordinator {
    fn drop(&mut self) {
        self.close().ok();
    }
}

#[async_trait(?Send)]
impl VirtualFileReader for FileWithBackgroundCoordinator {
    async fn read_next(&self) -> NativeExtensionsResult<Vec<u8>> {
        let mut file = self.file.borrow_mut();
        match file.as_mut() {
            Some(file) => {
                let mut buf = vec![0; 1024 * 1024];
                file.read(&mut buf)
                    .map(|size| {
                        buf.truncate(size);
                        buf
                    })
                    .map_err(NativeExtensionsError::from)
            }
            None => {
                return Err(NativeExtensionsError::VirtualFileReceiveError(
                    "File already closed".into(),
                ));
            }
        }
    }

    fn file_size(&self) -> NativeExtensionsResult<Option<i64>> {
        let file = self.file.borrow();
        match file.as_ref() {
            Some(file) => {
                let metadata = file.metadata()?;
                Ok(Some(metadata.len() as i64))
            }
            None => Ok(None),
        }
    }

    fn file_name(&self) -> Option<String> {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
    }

    fn close(&self) -> NativeExtensionsResult<()> {
        let file = self.file.borrow_mut().take();
        if let Some(_file) = file {
            self.coordinator_thread_release.set(());
        }
        Ok(())
    }
}
