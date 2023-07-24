use std::{
    cell::RefCell,
    fs,
    path::PathBuf,
    rc::{Rc, Weak},
    sync::Arc,
    thread,
};

use block::ConcreteBlock;
use cocoa::{
    appkit::{NSPasteboard, NSPasteboardItem},
    base::{id, nil},
    foundation::{NSArray, NSUInteger, NSURL},
};

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
    platform_impl::platform::common::{
        format_from_url, from_nsstring, nserror_description, path_from_url, to_nsdata, to_nsstring,
        uti_conforms_to,
    },
    reader_manager::{ReadProgress, VirtualFileReader},
};

use super::PlatformDataProvider;

pub struct PlatformDataReader {
    pasteboard: StrongPtr,
    promise_receivers: RefCell<Vec<Option<StrongPtr>>>,
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

    pub fn get_items_sync(&self) -> NativeExtensionsResult<Vec<i64>> {
        let count = autoreleasepool(|| unsafe {
            let items: id = msg_send![*self.pasteboard, pasteboardItems];
            NSArray::count(items) as i64
        });
        Ok((0..count).collect())
    }

    pub async fn get_items(&self) -> NativeExtensionsResult<Vec<i64>> {
        self.get_items_sync()
    }

    fn promise_receiver_types_for_item(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        autoreleasepool(|| unsafe {
            let items: id = msg_send![*self.pasteboard, pasteboardItems];
            if item < NSArray::count(items) as i64 {
                let pasteboard_item = NSArray::objectAtIndex(items, item as NSUInteger);
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
                    let ty = to_nsstring("com.apple.pasteboard.promised-file-content-type");
                    let value = NSPasteboardItem::stringForType(pasteboard_item, *ty);
                    if value != nil {
                        let string = from_nsstring(value);
                        if !string.is_empty() {
                            push(&mut res, string);
                        }
                    }
                    let receiver_types: id = msg_send![*receiver, fileTypes];
                    for i in 0..NSArray::count(receiver_types) {
                        push(
                            &mut res,
                            from_nsstring(NSArray::objectAtIndex(receiver_types, i)),
                        );
                    }
                }
                Ok(res)
            } else {
                Ok(Vec::new())
            }
        })
    }

    pub fn get_formats_for_item_sync(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        autoreleasepool(|| unsafe {
            let items: id = msg_send![*self.pasteboard, pasteboardItems];
            if item < NSArray::count(items) as i64 {
                let pasteboard_item = NSArray::objectAtIndex(items, item as NSUInteger);
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
                let types = NSPasteboardItem::types(pasteboard_item);
                for i in 0..NSArray::count(types) {
                    let format = from_nsstring(NSArray::objectAtIndex(types, i));
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
        })
    }

    fn needs_to_synthesize_png(&self, item: i64) -> bool {
        autoreleasepool(|| unsafe {
            let items: id = msg_send![*self.pasteboard, pasteboardItems];
            let mut has_tiff = false;
            let mut has_png = false;
            if item < NSArray::count(items) as i64 {
                let item = NSArray::objectAtIndex(items, item as NSUInteger);
                let types = NSPasteboardItem::types(item);
                for i in 0..NSArray::count(types) {
                    let format = from_nsstring(NSArray::objectAtIndex(types, i));
                    has_tiff |= format == "public.tiff";
                    has_png |= format == "public.png";
                }
            }
            has_tiff && !has_png
        })
    }

    pub fn item_format_is_synthesized(
        &self,
        item: i64,
        format: &str,
    ) -> NativeExtensionsResult<bool> {
        Ok(format == "public.png" && self.needs_to_synthesize_png(item))
    }

    fn item_has_virtual_file(&self, item: i64) -> bool {
        autoreleasepool(|| unsafe {
            let items: id = msg_send![*self.pasteboard, pasteboardItems];
            if item < NSArray::count(items) as i64 {
                let item = NSArray::objectAtIndex(items, item as NSUInteger);
                let types = NSPasteboardItem::types(item);
                for i in 0..NSArray::count(types) {
                    let format = from_nsstring(NSArray::objectAtIndex(types, i));
                    if format == "com.apple.NSFilePromiseItemMetaData"
                        || format == "com.apple.pasteboard.promised-file-url"
                    {
                        return true;
                    }
                }
            }
            false
        })
    }

    fn get_promise_receiver_for_item(
        &self,
        item: i64,
    ) -> NativeExtensionsResult<Option<StrongPtr>> {
        autoreleasepool(|| {
            if self.promise_receivers.borrow().is_empty() {
                let class = class!(NSFilePromiseReceiver) as *const _ as id;
                let receivers: id = unsafe {
                    msg_send![*self.pasteboard,
                        readObjectsForClasses: NSArray::arrayWithObject(nil, class) options:nil]
                };
                let mut receiver_index: NSUInteger = 0;
                let items = self.get_items_sync()?;
                for item in items {
                    if receiver_index < unsafe { NSArray::count(receivers) }
                        && self.item_has_virtual_file(item)
                    {
                        let receiver = unsafe {
                            StrongPtr::retain(NSArray::objectAtIndex(receivers, receiver_index))
                        };
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
        })
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
            let names: id = unsafe { msg_send![*receiver, fileNames] };
            let len = unsafe { NSArray::count(names) };
            if len > 0 {
                let name = unsafe { from_nsstring(NSArray::objectAtIndex(names, 0)) };
                return Ok(Some(name));
            }
        }
        let data = self
            .do_get_data_for_item(item, "public.file-url".to_owned())
            .await?;
        if let Some(url) = Self::value_to_string(data) {
            let url = unsafe { NSURL::URLWithString_(nil, *to_nsstring(&url)) };
            let path = path_from_url(url);
            return Ok(path.file_name().map(|f| f.to_string_lossy().to_string()));
        }

        let data = self
            .do_get_data_for_item(item, "public.url".to_owned())
            .await?;
        if let Some(url) = Self::value_to_string(data) {
            let url = unsafe { NSURL::URLWithString_(nil, *to_nsstring(&url)) };
            let name: id = unsafe { msg_send![url, lastPathComponent] };
            if !name.is_null() {
                return Ok(Some(unsafe { from_nsstring(name) }));
            }
        }

        Ok(None)
    }

    pub async fn convert_to_png(&self, data: Vec<u8>) -> NativeExtensionsResult<Value> {
        let (future, completer) = FutureCompleter::new();
        let mut completer = Capsule::new(completer);
        let sender = RunLoop::current().new_sender();
        thread::spawn(move || {
            autoreleasepool(|| unsafe {
                let data = to_nsdata(&data);
                let rep: id = msg_send![class!(NSBitmapImageRep), imageRepWithData:*data];
                let type_png: NSUInteger = 4;
                let png: id = msg_send![rep, representationUsingType:type_png properties:nil];
                let res = Value::from_objc(png).ok_log().unwrap_or_default();
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
        pasteboard: StrongPtr,
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
                let data = autoreleasepool(|| unsafe {
                    let items: id = msg_send![*pasteboard, pasteboardItems];
                    if item < NSArray::count(items) as i64 {
                        let item = NSArray::objectAtIndex(items, item as NSUInteger);
                        let is_file_url = data_type == "public.file-url";
                        let is_text = uti_conforms_to(&data_type, "public.text");
                        let data_type = to_nsstring(&data_type);
                        // Try to get property list first, otherwise fallback to Data
                        let mut data = if is_text {
                            NSPasteboardItem::stringForType(item, *data_type)
                        } else {
                            NSPasteboardItem::propertyListForType(item, *data_type)
                        };
                        if data.is_null() {
                            // Ask for data here. It's better for Appkit to convert String to data,
                            // then trying to convert data to String.
                            data = NSPasteboardItem::dataForType(item, *data_type);
                        }
                        let res = Value::from_objc(data).ok_log().unwrap_or_default();
                        // Convert file:///.file/id=??? URLs to path URL
                        if is_file_url {
                            if let Value::String(url) = res {
                                let url = NSURL::URLWithString_(nil, *to_nsstring(&url));
                                let url: id = msg_send![url, filePathURL];
                                let url_string: id = msg_send![url, absoluteString];
                                return Value::String(from_nsstring(url_string));
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
            StrongPtr::retain(NSPasteboard::generalPasteboard(nil))
        }))
    }

    pub fn from_pasteboard(pasteboard: StrongPtr) -> Rc<Self> {
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
                let res = autoreleasepool(|| {
                    let target_folder = target_folder.to_string_lossy();
                    let url = unsafe { NSURL::fileURLWithPath_(nil, *to_nsstring(&target_folder)) };
                    let queue: id = unsafe { msg_send![class!(NSOperationQueue), mainQueue] };
                    let (future, completer) = FutureCompleter::new();
                    let completer = Rc::new(RefCell::new(Some(completer)));
                    let block = ConcreteBlock::new(move |url: id, error: id| {
                        let completer = completer
                            .borrow_mut()
                            .take()
                            .expect("Callback invoked more than once");
                        if error != nil {
                            if url != nil {
                                fs::remove_file(path_from_url(url)).ok_log();
                            }
                            completer.complete(Err(NativeExtensionsError::VirtualFileReceiveError(
                                nserror_description(error),
                            )))
                        } else {
                            completer.complete(Ok(path_from_url(url)))
                        }
                    });
                    let block = block.copy();
                    let () = unsafe {
                        msg_send![*receiver,
                                receivePromisedFilesAtDestination: url
                                options: nil
                                operationQueue: queue
                                reader: &*block]
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
