use std::{
    cell::RefCell,
    fs,
    path::PathBuf,
    ptr::NonNull,
    rc::{Rc, Weak},
    sync::Arc,
    thread,
};

use icrate::{
    block2::ConcreteBlock,
    ns_string,
    AppKit::{NSBitmapImageFileTypePNG, NSBitmapImageRep, NSFilePromiseReceiver, NSPasteboard},
    Foundation::{NSArray, NSData, NSDictionary, NSError, NSOperationQueue, NSString, NSURL},
};
use irondash_message_channel::{value_darwin::ValueObjcConversion, Value};
use irondash_run_loop::{
    util::{Capsule, FutureCompleter},
    RunLoop,
};
use objc2::{
    msg_send_id,
    rc::{Id, autoreleasepool},
    runtime::{AnyObject, NSObject},
    ClassType,
};

use crate::{
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    platform_impl::platform::common::{format_from_url, path_from_url, uti_conforms_to},
    reader_manager::{ReadProgress, VirtualFileReader},
};

use super::PlatformDataProvider;

pub struct PlatformDataReader {
    pasteboard: Id<NSPasteboard>,
    promise_receivers: RefCell<Vec<Option<Id<NSFilePromiseReceiver>>>>,
}

impl PlatformDataReader {
    pub async fn get_format_for_file_uri(
        file_uri: String,
    ) -> NativeExtensionsResult<Option<String>> {
        let res = unsafe {
            let string = NSString::from_str(&file_uri);
            let url = NSURL::URLWithString(&string);
            url.and_then(|url| format_from_url(&url))
        };
        Ok(res)
    }

    pub fn get_items_sync(&self) -> NativeExtensionsResult<Vec<i64>> {
        let count = unsafe {
            let items = self.pasteboard.pasteboardItems();
            items.map(|items| items.count()).unwrap_or(0)
        };
        Ok((0..count as i64).collect())
    }

    pub async fn get_items(&self) -> NativeExtensionsResult<Vec<i64>> {
        self.get_items_sync()
    }

    fn promise_receiver_types_for_item(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        let items = unsafe { self.pasteboard.pasteboardItems() };
        let items = items.unwrap_or_default();
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

                for ty in receiver_types {
                    push(&mut res, ty.to_string());
                }
            }
            Ok(res)
        } else {
            Ok(Vec::new())
        }
    }

    pub fn get_formats_for_item_sync(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        let items = unsafe { self.pasteboard.pasteboardItems() }.unwrap_or_default();
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
        let items = unsafe { self.pasteboard.pasteboardItems() }.unwrap_or_default();
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
        let items = unsafe { self.pasteboard.pasteboardItems() }.unwrap_or_default();
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
                    NSBitmapImageFileTypePNG,
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
        item: i64,
        pasteboard: Id<NSPasteboard>,
        data_type: String,
        completer: FutureCompleter<Value>,
    ) {
        RunLoop::current()
            .schedule_next(move || {
                if PlatformDataProvider::is_waiting_for_pasteboard_data() {
                    // We're currently running nested run loop in which pasteboard is waiting
                    // for data. Trying to get data from pasteboard at this stage may lead to
                    // deadlock.
                    Self::schedule_do_get_data_for_item(
                        item,
                        pasteboard.clone(),
                        data_type,
                        completer,
                    );
                    return;
                }
                let data = autoreleasepool(|_| unsafe {
                    let items = pasteboard.pasteboardItems().unwrap_or_default();
                    if item < items.count() as i64 {
                        let item = items.objectAtIndex(item as usize);
                        let is_file_url = data_type == "public.file-url";
                        let is_text = uti_conforms_to(&data_type, "public.text");
                        let data_type = NSString::from_str(&data_type);
                        // Try to get property list first, otherwise fallback to Data
                        let mut data: Option<Id<NSObject>> = if is_text {
                            item.stringForType(&data_type).map(|i| Id::cast(i))
                        } else {
                            item.propertyListForType(&data_type).map(|i| Id::cast(i))
                        };
                        if data.is_none() {
                            // Ask for data here. It's better for Appkit to convert String to data,
                            // then trying to convert data to String.
                            data = item.dataForType(&data_type).map(|i| Id::cast(i));
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
                    } else {
                        Value::Null
                    }
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
        let (future, completer) = FutureCompleter::new();
        let pasteboard = self.pasteboard.clone();
        // Retrieving data may require call back to Flutter and nested run loop so don't
        // block current dispatch
        Self::schedule_do_get_data_for_item(item, pasteboard.clone(), data_type, completer);

        Ok(future.await)
    }

    pub fn new_clipboard_reader() -> NativeExtensionsResult<Rc<Self>> {
        Ok(Self::from_pasteboard(unsafe {
            NSPasteboard::generalPasteboard()
        }))
    }

    pub fn from_pasteboard(pasteboard: Id<NSPasteboard>) -> Rc<Self> {
        let res = Rc::new(Self {
            pasteboard,
            promise_receivers: RefCell::new(Vec::new()),
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
                    let block =
                        ConcreteBlock::new(move |url: NonNull<NSURL>, error: *mut NSError| {
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

                                completer.complete(Err(
                                    NativeExtensionsError::VirtualFileReceiveError(
                                        error.localizedDescription().to_string(),
                                    ),
                                ))
                            } else {
                                let url = url.unwrap();
                                completer.complete(Ok(path_from_url(&url)))
                            }
                        });
                    let block = block.copy();
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
