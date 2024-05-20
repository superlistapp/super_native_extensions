use std::{
    cell::RefCell,
    collections::HashMap,
    fs,
    path::PathBuf,
    ptr::NonNull,
    rc::{Rc, Weak},
    sync::Arc,
    thread,
};

use block2::RcBlock;
use irondash_message_channel::{value_darwin::ValueObjcConversion, Value};
use irondash_run_loop::{
    util::{Capsule, FutureCompleter},
    RunLoop,
};
use objc2::{
    msg_send_id,
    rc::{autoreleasepool, Id},
    runtime::{AnyObject, NSObject},
    ClassType,
};
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSFilePromiseReceiver, NSPasteboard, NSPasteboardItem,
};

use objc2_foundation::{
    ns_string, NSArray, NSData, NSDictionary, NSError, NSOperationQueue, NSString, NSURL,
};

use crate::{
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    platform_impl::platform::common::{format_from_url, path_from_url, uti_conforms_to},
    reader_manager::{ReadProgress, VirtualFileReader},
};

use super::PlatformDataProvider;

#[derive(Hash, Eq, PartialEq)]
struct ValueCacheKey {
    item: i64,
    format: String,
}

pub struct PlatformDataReader {
    pasteboard: Id<NSPasteboard>,
    pasteboard_items: RefCell<Option<Id<NSArray<NSPasteboardItem>>>>,
    promise_receivers: RefCell<Vec<Option<Id<NSFilePromiseReceiver>>>>,
    cached_formats: RefCell<HashMap<i64, Vec<String>>>,
    value_cache: RefCell<HashMap<ValueCacheKey, Value>>,
}

impl PlatformDataReader {
    fn get_pasteboard_items(&self) -> NativeExtensionsResult<Id<NSArray<NSPasteboardItem>>> {
        let items = self.pasteboard_items.clone().take();
        if let Some(items) = items {
            Ok(items)
        } else {
            let items = unsafe { self.pasteboard.pasteboardItems() }.unwrap_or_default();
            self.pasteboard_items.replace(Some(items.clone()));
            Ok(items)
        }
    }

    pub fn get_items_sync(&self) -> NativeExtensionsResult<Vec<i64>> {
        let count = self.get_pasteboard_items()?.count();
        Ok((0..count as i64).collect())
    }

    pub async fn get_items(&self) -> NativeExtensionsResult<Vec<i64>> {
        self.get_items_sync()
    }

    pub async fn get_item_format_for_uri(
        &self,
        item: i64,
    ) -> NativeExtensionsResult<Option<String>> {
        let data = self
            .get_data_for_item(item, "public.file-url".to_owned(), None)
            .await?;
        if let Value::String(file_uri) = data {
            let string = NSString::from_str(&file_uri);
            let url = unsafe { NSURL::URLWithString(&string) };
            Ok(url.and_then(|url| unsafe { format_from_url(&url) }))
        } else {
            Ok(None)
        }
    }

    fn promise_receiver_types_for_item(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        let items = self.get_pasteboard_items()?;
        if item < items.count() as i64 {
            let pasteboard_item = unsafe { items.objectAtIndex(item as usize) };
            let mut res = Vec::new();
            fn push(res: &mut Vec<String>, s: String) {
                if !res.contains(&s) {
                    res.push(s);
                }
            }
            // First virtual files
            let receiver = self.get_promise_receiver_for_item(item)?;
            if let Some(receiver) = receiver {
                // Outlook reports wrong types for [fileTypes] (extension instead of UTI), but has correct type
                // in "com.apple.pasteboard.promised-file-content-type.
                let ty = ns_string!("com.apple.pasteboard.promised-file-content-type");
                let value = unsafe { pasteboard_item.stringForType(ty) };
                if let Some(value) = value {
                    let string = value.to_string();
                    if !string.is_empty() {
                        push(&mut res, string);
                    }
                }
                let receiver_types = unsafe { receiver.fileTypes() };

                for ty in receiver_types.iter() {
                    push(&mut res, ty.to_string());
                }
            }
            Ok(res)
        } else {
            Ok(Vec::new())
        }
    }

    pub fn get_formats_for_item_sync(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        let mut cached_formats = self.cached_formats.borrow_mut();
        if let Some(formats) = cached_formats.get(&item).cloned() {
            return Ok(formats);
        }
        let formats = self._get_formats_for_item_sync(item)?;
        cached_formats.insert(item, formats.clone());
        Ok(formats)
    }

    pub fn _get_formats_for_item_sync(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        let items = self.get_pasteboard_items()?;
        if item < items.count() as i64 {
            let pasteboard_item = unsafe { items.objectAtIndex(item as usize) };
            let mut res = Vec::new();
            fn push(res: &mut Vec<String>, s: String) {
                if !res.contains(&s) {
                    res.push(s);
                }
            }
            // First virtual files
            let virtual_types = self.promise_receiver_types_for_item(item)?;
            for format in virtual_types {
                push(&mut res, format);
            }
            // Second regular items
            let types = unsafe { pasteboard_item.types() };
            for format in types {
                let format = format.to_string();
                push(&mut res, format.clone());
                // Put synthesized PNG right after tiff
                if format == "public.tiff" && self.needs_to_synthesize_png(item) {
                    res.push("public.png".to_string());
                }
            }

            Ok(res)
        } else {
            Ok(Vec::new())
        }
    }

    fn needs_to_synthesize_png(&self, item: i64) -> bool {
        let Ok(items) = self.get_pasteboard_items() else {
            return false;
        };
        let mut has_tiff = false;
        let mut has_png = false;
        if item < items.count() as i64 {
            let item = unsafe { items.objectAtIndex(item as usize) };
            let types = unsafe { item.types() };
            for format in types {
                let format = format.to_string();
                has_tiff |= format == "public.tiff";
                has_png |= format == "public.png";
            }
        }
        has_tiff && !has_png
    }

    pub fn item_format_is_synthesized(
        &self,
        item: i64,
        format: &str,
    ) -> NativeExtensionsResult<bool> {
        Ok(format == "public.png" && self.needs_to_synthesize_png(item))
    }

    fn item_has_virtual_file(&self, item: i64) -> bool {
        let Ok(items) = self.get_pasteboard_items() else {
            return false;
        };
        if item < items.count() as i64 {
            let item = unsafe { items.objectAtIndex(item as usize) };
            let types = unsafe { item.types() };
            for iformat in types {
                let format = iformat.to_string();
                if format == "com.apple.NSFilePromiseItemMetaData"
                    || format == "com.apple.pasteboard.promised-file-url"
                {
                    return true;
                }
            }
        }
        false
    }

    fn get_promise_receiver_for_item(
        &self,
        item: i64,
    ) -> NativeExtensionsResult<Option<Id<NSFilePromiseReceiver>>> {
        if self.promise_receivers.borrow().is_empty() {
            let class =
                unsafe { Id::retain(NSFilePromiseReceiver::class() as *const _ as *mut AnyObject) }
                    .unwrap();
            let receivers = unsafe {
                self.pasteboard
                    .readObjectsForClasses_options(&NSArray::from_vec(vec![Id::cast(class)]), None)
            }
            .unwrap_or_default();
            let mut receiver_index = 0usize;
            let items = self.get_items_sync()?;
            for item in items {
                if receiver_index < receivers.count() && self.item_has_virtual_file(item) {
                    let receiver = unsafe { receivers.objectAtIndex(receiver_index) };
                    let receiver = unsafe { Id::cast::<NSFilePromiseReceiver>(receiver) };
                    receiver_index += 1;
                    self.promise_receivers.borrow_mut().push(Some(receiver));
                } else {
                    self.promise_receivers.borrow_mut().push(None);
                }
            }
        }
        let res = self
            .promise_receivers
            .borrow()
            .get(item as usize)
            .and_then(|a| a.as_ref().cloned());
        Ok(res)
    }

    pub async fn get_formats_for_item(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        self.get_formats_for_item_sync(item)
    }

    fn value_to_string(value: Value) -> Option<String> {
        match value {
            Value::String(string) => Some(string),
            Value::U8List(list) => Some(String::from_utf8_lossy(&list).to_string()),
            _ => None,
        }
    }

    pub async fn get_suggested_name_for_item(
        &self,
        item: i64,
    ) -> NativeExtensionsResult<Option<String>> {
        let receiver = self.get_promise_receiver_for_item(item)?;
        if let Some(receiver) = receiver {
            // fileNames is actually can be null :-/
            let names: Option<Id<NSArray<NSString>>> =
                unsafe { msg_send_id![&receiver, fileNames] };
            if let Some(names) = names {
                let name = names.iter().next();
                if let Some(name) = name {
                    return Ok(Some(name.to_string()));
                }
            }
        }

        for ty in ["public.file-url", "public.url"] {
            let data = self.do_get_data_for_item(item, ty.to_owned()).await?;
            if let Some(url) = Self::value_to_string(data) {
                let url = unsafe { NSURL::URLWithString(&NSString::from_str(&url)) };
                let name = url.and_then(|url| unsafe { url.lastPathComponent() });
                if let Some(name) = name {
                    return Ok(Some(name.to_string()));
                }
            }
        }

        Ok(None)
    }

    pub async fn convert_to_png(&self, data: Vec<u8>) -> NativeExtensionsResult<Value> {
        let (future, completer) = FutureCompleter::new();
        let mut completer = Capsule::new(completer);
        let sender = RunLoop::current().new_sender();
        thread::spawn(move || {
            autoreleasepool(|_| unsafe {
                let data = NSData::from_vec(data);
                let rep = NSBitmapImageRep::imageRepWithData(&data).unwrap();
                let png = rep.representationUsingType_properties(
                    NSBitmapImageFileType::PNG,
                    &NSDictionary::dictionary(),
                );

                let res = Value::from_objc(png.map(|png| Id::cast(png)))
                    .ok_log()
                    .unwrap_or_default();
                sender.send(move || {
                    let completer = completer.take().unwrap();
                    completer.complete(Ok(res));
                });
            });
        });
        future.await
    }

    pub async fn get_data_for_item(
        &self,
        item: i64,
        data_type: String,
        _progress: Option<Arc<ReadProgress>>,
    ) -> NativeExtensionsResult<Value> {
        if data_type == "public.png" && self.needs_to_synthesize_png(item) {
            let tiff = self
                .do_get_data_for_item(item, "public.tiff".to_owned())
                .await?;
            match tiff {
                Value::U8List(data) => self.convert_to_png(data).await,
                other => Ok(other),
            }
        } else {
            self.do_get_data_for_item(item, data_type).await
        }
    }

    fn schedule_do_get_data_for_item(
        item: Id<NSPasteboardItem>,
        data_type: String,
        completer: FutureCompleter<Value>,
    ) {
        RunLoop::current()
            .schedule_next(move || {
                if PlatformDataProvider::is_waiting_for_pasteboard_data() {
                    // We're currently running nested run loop in which pasteboard is waiting
                    // for data. Trying to get data from pasteboard at this stage may lead to
                    // deadlock.
                    Self::schedule_do_get_data_for_item(item.clone(), data_type, completer);
                    return;
                }
                let data = autoreleasepool(|_| unsafe {
                    let pasteboard_item = item;
                    let is_file_url = data_type == "public.file-url";
                    let is_text = uti_conforms_to(&data_type, "public.text");
                    let data_type = NSString::from_str(&data_type);
                    // Try to get property list first, otherwise fallback to Data
                    let mut data: Option<Id<NSObject>> = if is_text || is_file_url {
                        pasteboard_item
                            .stringForType(&data_type)
                            .map(|i| Id::cast(i))
                    } else {
                        pasteboard_item
                            .propertyListForType(&data_type)
                            .map(|i| Id::cast(i))
                    };
                    if data.is_none() {
                        // Ask for data here. It's better for Appkit to convert String to data,
                        // then trying to convert data to String.
                        data = pasteboard_item.dataForType(&data_type).map(|i| Id::cast(i));
                    }
                    let res = Value::from_objc(data).ok_log().unwrap_or_default();
                    // Convert file:///.file/id=??? URLs to path URL
                    if is_file_url {
                        if let Value::String(url) = &res {
                            let url = NSURL::URLWithString(&NSString::from_str(url));
                            let url = url.and_then(|url| url.filePathURL());
                            if let Some(url) = url {
                                let string = url.absoluteString().unwrap();
                                return Value::String(string.to_string());
                            }
                        }
                    }
                    res
                });
                completer.complete(data)
            })
            .detach();
    }

    async fn do_get_data_for_item(
        &self,
        item: i64,
        data_type: String,
    ) -> NativeExtensionsResult<Value> {
        let cache_key = ValueCacheKey {
            item,
            format: data_type.clone(),
        };
        {
            let value_cache = self.value_cache.borrow();
            if let Some(value) = value_cache.get(&cache_key).cloned() {
                return Ok(value);
            }
        }

        let (future, completer) = FutureCompleter::new();
        // Retrieving data may require call back to Flutter and nested run loop so don't
        // block current dispatch
        let pasteboard_item = unsafe { self.get_pasteboard_items()?.objectAtIndex(item as usize) };
        Self::schedule_do_get_data_for_item(pasteboard_item, data_type, completer);

        let res = future.await;
        let mut value_cache = self.value_cache.borrow_mut();
        value_cache.insert(cache_key, res.clone());
        Ok(res)
    }

    pub fn new_clipboard_reader() -> NativeExtensionsResult<Rc<Self>> {
        Ok(Self::from_pasteboard(unsafe {
            NSPasteboard::generalPasteboard()
        }))
    }

    pub fn from_pasteboard(pasteboard: Id<NSPasteboard>) -> Rc<Self> {
        let res = Rc::new(Self {
            pasteboard,
            pasteboard_items: RefCell::new(None),
            promise_receivers: RefCell::new(Vec::new()),
            cached_formats: RefCell::new(HashMap::new()),
            value_cache: RefCell::new(HashMap::new()),
        });
        res.assign_weak_self(Rc::downgrade(&res));
        res
    }

    pub async fn can_copy_virtual_file_for_item(
        &self,
        item: i64,
        format: &str,
    ) -> NativeExtensionsResult<bool> {
        let virtual_types = self.promise_receiver_types_for_item(item)?;
        Ok(virtual_types.iter().any(|f| f == format))
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
        _format: &str,
        target_folder: PathBuf,
        _progress: Arc<ReadProgress>,
    ) -> NativeExtensionsResult<PathBuf> {
        let receiver = self.get_promise_receiver_for_item(item)?;
        match receiver {
            Some(receiver) => {
                let res = autoreleasepool(|_| {
                    let target_folder = target_folder.to_string_lossy();
                    let url =
                        unsafe { NSURL::fileURLWithPath(&NSString::from_str(&target_folder)) };
                    let queue = unsafe { NSOperationQueue::mainQueue() };
                    let (future, completer) = FutureCompleter::new();
                    let completer = Rc::new(RefCell::new(Some(completer)));
                    let block = RcBlock::new(move |url: NonNull<NSURL>, error: *mut NSError| {
                        let url = unsafe { Id::retain(url.as_ptr()) };
                        let error = unsafe { Id::retain(error) };
                        let completer = completer
                            .borrow_mut()
                            .take()
                            .expect("Callback invoked more than once");
                        if let Some(error) = error {
                            if let Some(url) = url {
                                fs::remove_file(path_from_url(&url)).ok_log();
                            }

                            completer.complete(Err(NativeExtensionsError::VirtualFileReceiveError(
                                error.localizedDescription().to_string(),
                            )))
                        } else {
                            let url = url.unwrap();
                            completer.complete(Ok(path_from_url(&url)))
                        }
                    });
                    unsafe {
                        receiver.receivePromisedFilesAtDestination_options_operationQueue_reader(
                            &url,
                            &NSDictionary::dictionary(),
                            &queue,
                            &block,
                        )
                    };
                    future
                });
                res.await
            }
            None => Err(NativeExtensionsError::OtherError(
                "FilePromiseReceiver is not available".to_owned(),
            )),
        }
    }

    pub fn assign_weak_self(&self, _weak: Weak<PlatformDataReader>) {}
}
