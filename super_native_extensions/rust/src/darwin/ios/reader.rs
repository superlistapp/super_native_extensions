use std::{
    fs,
    path::PathBuf,
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
};

use block::ConcreteBlock;
use cocoa::{
    base::{id, nil},
    foundation::{NSArray, NSURL},
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
                // travels between threads, must be refcounted because lock is Fn
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
        // loadFileRepresentationForTypeIdentifier so we add a bit of heurists
        // to avoid creating a temporary file for text, composite content, images
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
        _item: i64,
        _format: &str,
    ) -> NativeExtensionsResult<bool> {
        Ok(false)
    }

    pub async fn create_virtual_file_reader_for_item(
        &self,
        _item: i64,
        _format: &str,
        _progress: Arc<ReadProgress>,
    ) -> NativeExtensionsResult<Option<Rc<dyn VirtualFileReader>>> {
        Ok(None)
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
            if item < providers.len() as i64 {
                // travels between threads, must be refcounted because lock is Fn
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
                let ns_progress: id = msg_send![provider, loadFileRepresentationForTypeIdentifier:*to_nsstring(format)
                                      completionHandler:&*block];
                bridge_progress(ns_progress, read_progress);
            } else {
                completer.complete(Err(NativeExtensionsError::OtherError(
                    "Invalid item".into(),
                )));
            }
        });
        future.await
    }

    pub fn assign_weak_self(&self, _weak: Weak<PlatformDataReader>) {}
}
