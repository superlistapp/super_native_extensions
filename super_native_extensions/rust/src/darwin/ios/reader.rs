use std::{
    cell::RefCell,
    fs::{self, File},
    io::Read,
    path::PathBuf,
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
    thread,
};

use async_trait::async_trait;
use block::ConcreteBlock;
use cocoa::{
    base::{id, nil, BOOL},
    foundation::{NSArray, NSUInteger, NSURL},
};

use core_foundation::{base::TCFType, string::CFString};
use irondash_message_channel::{value_darwin::ValueObjcConversion, Value};
use irondash_run_loop::{
    util::{Capsule, FutureCompleter},
    RunLoop,
};

use objc::{
    class, msg_send,
    rc::{autoreleasepool, StrongPtr},
    sel, sel_impl,
};

use crate::{
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    platform_impl::platform::{
        common::{
            format_from_url, from_nsstring, nserror_description, path_from_url, to_nsstring,
            UTTypeConformsTo,
        },
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
    Pasteboard(StrongPtr),
    DropSessionItems(StrongPtr),
}

impl PlatformDataReader {
    pub async fn get_format_for_file_uri(
        file_uri: String,
    ) -> NativeExtensionsResult<Option<String>> {
        let res = autoreleasepool(|| unsafe {
            let string = to_nsstring(&file_uri);
            let url = NSURL::URLWithString_(nil, *string);
            format_from_url(url)
        });
        Ok(res)
    }

    fn get_items_providers(&self) -> Vec<id> {
        match &self.source {
            ReaderSource::Pasteboard(pasteboard) => {
                let providers: id = unsafe { msg_send![**pasteboard, itemProviders] };
                (0..unsafe { NSArray::count(providers) })
                    .map(|i| unsafe { NSArray::objectAtIndex(providers, i) })
                    .collect()
            }
            ReaderSource::DropSessionItems(items) => (0..unsafe { NSArray::count(**items) })
                .map(|i| {
                    let item = unsafe { NSArray::objectAtIndex(**items, i) };
                    unsafe { msg_send![item, itemProvider] }
                })
                .collect(),
        }
    }

    pub async fn get_items(&self) -> NativeExtensionsResult<Vec<i64>> {
        let count = autoreleasepool(|| {
            let providers = self.get_items_providers();
            providers.len() as i64
        });
        Ok((0..count).collect())
    }

    pub fn get_formats_for_item_sync(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        let formats = autoreleasepool(|| unsafe {
            let providers = self.get_items_providers();
            if item < providers.len() as i64 {
                let provider = providers[item as usize];
                let identifiers: id = msg_send![provider, registeredTypeIdentifiers];
                (0..NSArray::count(identifiers))
                    .map(|i| from_nsstring(NSArray::objectAtIndex(identifiers, i)))
                    .collect()
            } else {
                Vec::new()
            }
        });
        Ok(formats)
    }

    pub async fn get_formats_for_item(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        self.get_formats_for_item_sync(item)
    }

    pub async fn get_suggested_name_for_item(
        &self,
        item: i64,
    ) -> NativeExtensionsResult<Option<String>> {
        let name = autoreleasepool(|| unsafe {
            let providers = self.get_items_providers();
            if item < providers.len() as i64 {
                let provider = providers[item as usize];
                let name: id = msg_send![provider, suggestedName];
                if name.is_null() {
                    None
                } else {
                    Some(from_nsstring(name))
                }
            } else {
                None
            }
        });
        Ok(name)
    }

    unsafe fn maybe_decode_bplist(data: id) -> id {
        let bytes: *const u8 = msg_send![data, bytes];
        let length: usize = msg_send![data, length];
        let data_slice: &[u8] = std::slice::from_raw_parts(bytes, length);
        let magic: &[u8; 8] = &[98, 112, 108, 105, 115, 116, 48, 48];
        if data_slice.starts_with(magic) {
            let list: id = msg_send![class!(NSPropertyListSerialization), propertyListWithData:data options:0 format:nil error:nil];
            if list != nil {
                list
            } else {
                data
            }
        } else {
            data
        }
    }

    pub async fn get_data_for_item(
        &self,
        item: i64,
        format: String,
        read_progress: Option<Arc<ReadProgress>>,
    ) -> NativeExtensionsResult<Value> {
        let (future, completer) = FutureCompleter::new();
        autoreleasepool(|| unsafe {
            let providers = self.get_items_providers();
            if item < providers.len() as i64 {
                // travels between threads, must be refcounted because block is Fn
                let completer = Arc::new(Mutex::new(Capsule::new(completer)));
                let provider = providers[item as usize];
                let sender = RunLoop::current().new_sender();
                let block = ConcreteBlock::new(move |data: id, _err: id| {
                    let data = Self::maybe_decode_bplist(data);
                    let data = Movable::new(StrongPtr::retain(data));
                    let completer = completer.clone();
                    sender.send(move || {
                        let completer = completer
                            .lock()
                            .unwrap()
                            .take()
                            .expect("Block invoked more than once");
                        let data = data;
                        completer.complete(Ok(Value::from_objc(*data.take())
                            .ok_log()
                            .unwrap_or_default()))
                    });
                });
                let block = block.copy();
                let ns_progress: id = msg_send![provider, loadDataRepresentationForTypeIdentifier:*to_nsstring(&format)
                                      completionHandler:&*block];
                if let Some(read_progress) = read_progress {
                    bridge_progress(ns_progress, read_progress);
                }
            } else {
                completer.complete(Ok(Value::Null));
            }
        });
        future.await
    }

    pub fn new_clipboard_reader() -> NativeExtensionsResult<Rc<Self>> {
        let res = Rc::new(Self {
            source: ReaderSource::Pasteboard(unsafe {
                StrongPtr::retain(msg_send![class!(UIPasteboard), generalPasteboard])
            }),
        });
        res.assign_weak_self(Rc::downgrade(&res));
        Ok(res)
    }

    pub fn new_with_drop_session_items(items: id) -> NativeExtensionsResult<Rc<Self>> {
        let res = Rc::new(Self {
            source: ReaderSource::DropSessionItems(unsafe { StrongPtr::retain(items) }),
        });
        res.assign_weak_self(Rc::downgrade(&res));
        Ok(res)
    }

    pub fn item_format_is_synthetized(
        &self,
        _item: i64,
        _format: &str,
    ) -> NativeExtensionsResult<bool> {
        Ok(false)
    }

    fn uti_conforms_to(uti: &str, conforms_to: &str) -> bool {
        let uti = CFString::new(uti);
        let conforms_to = CFString::new(conforms_to);

        let conforms_to = unsafe {
            UTTypeConformsTo(uti.as_concrete_TypeRef(), conforms_to.as_concrete_TypeRef())
        };

        conforms_to != 0
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
        if Self::uti_conforms_to(format, "public.composite-content")
            || Self::uti_conforms_to(format, "public.text")
            || Self::uti_conforms_to(format, "public.image")
            || Self::uti_conforms_to(format, "public.url")
            || Self::uti_conforms_to(format, "com.apple.property-list")
        {
            return Ok(false);
        }
        let formats = self.get_formats_for_item_sync(item)?;
        Ok(formats.iter().any(|f| f == format))
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
        let (future, completer) = FutureCompleter::new();
        autoreleasepool(|| unsafe {
            let providers = self.get_items_providers();
            if item >= providers.len() as i64 {
                completer.complete(Err(NativeExtensionsError::OtherError(
                    "Invalid item".into(),
                )));
                return;
            }
            // travels between threads, must be refcounted because block is Fn
            let completer = Arc::new(Mutex::new(Capsule::new(completer)));
            let provider = providers[item as usize];
            let sender = RunLoop::current().new_sender();
            let block = ConcreteBlock::new(move |url: id, _is_in_place: BOOL, err: id| {
                let res = if err != nil {
                    Err(NativeExtensionsError::VirtualFileReceiveError(
                        nserror_description(err),
                    ))
                } else {
                    FileWithBackgroundCoordinator::new(url)
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
            });
            let block = block.copy();
            let ns_progress: id = msg_send![provider,
                                    loadInPlaceFileRepresentationForTypeIdentifier:*to_nsstring(format)
                                    completionHandler:&*block];
            bridge_progress(ns_progress, read_progress);
        });
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
        autoreleasepool(|| unsafe {
            let providers = self.get_items_providers();
            if item >= providers.len() as i64 {
                completer.complete(Err(NativeExtensionsError::OtherError(
                    "Invalid item".into(),
                )));
                return;
            }
            // travels between threads, must be refcounted because block is Fn
            let completer = Arc::new(Mutex::new(Capsule::new(completer)));
            let provider = providers[item as usize];
            let sender = RunLoop::current().new_sender();
            let block = ConcreteBlock::new(move |url: id, err: id| {
                let res = if err != nil {
                    Err(NativeExtensionsError::VirtualFileReceiveError(
                        nserror_description(err),
                    ))
                } else {
                    let source_path = path_from_url(url);
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
            let block = block.copy();
            let ns_progress: id = msg_send![provider,
                                    loadFileRepresentationForTypeIdentifier:*to_nsstring(format)
                                    completionHandler:&*block];
            bridge_progress(ns_progress, read_progress);
        });
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
    fn new(url: id) -> NativeExtensionsResult<FileWithBackgroundCoordinator> {
        let promise = Arc::new(Promise::new());
        let url = unsafe { Movable::new(url) };
        let promise_clone = promise.clone();
        let promise_clone2 = promise.clone();
        thread::spawn(move || {
            let block = ConcreteBlock::new(move |new_url: id| {
                let path = path_from_url(new_url);
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
            let block = block.copy();
            let mut error: id = nil;
            autoreleasepool(|| unsafe {
                let coordinator: id = msg_send![class!(NSFileCoordinator), alloc];
                let coordinator: id = msg_send![coordinator, initWithFilePresenter: nil];
                let coordinator = StrongPtr::new(coordinator);
                let () = msg_send![*coordinator,
                    coordinateReadingItemAtURL:*url
                    options:1 as NSUInteger /* NSFileCoordinatorReadingWithoutChanges */
                    error: &mut error
                    byAccessor: &*block
                ];
                if error != nil {
                    promise_clone.set(Err(NativeExtensionsError::VirtualFileReceiveError(
                        nserror_description(error),
                    )));
                }
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
