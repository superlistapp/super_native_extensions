use std::rc::Weak;

use cocoa::{
    appkit::{NSPasteboard, NSPasteboardItem},
    base::{id, nil},
    foundation::{NSArray, NSUInteger},
};
use nativeshell_core::{
    platform::value::ValueObjcConversion, util::FutureCompleter, Context, Value,
};
use objc::{
    msg_send,
    rc::{autoreleasepool, StrongPtr},
    sel, sel_impl,
};

use crate::{error::NativeExtensionsResult, log::OkLog};

use super::util::{from_nsstring, to_nsstring};

pub struct PlatformClipboardReader {
    pasteboard: StrongPtr,
}

impl PlatformClipboardReader {
    pub async fn get_items(&self) -> NativeExtensionsResult<Vec<i64>> {
        let count = autoreleasepool(|| unsafe {
            let items: id = msg_send![*self.pasteboard, pasteboardItems];
            NSArray::count(items) as i64
        });
        Ok((0..count).collect())
    }

    pub async fn get_types_for_item(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        let types = autoreleasepool(|| unsafe {
            let items: id = msg_send![*self.pasteboard, pasteboardItems];
            if item < NSArray::count(items) as i64 {
                let item = NSArray::objectAtIndex(items, item as NSUInteger);
                let types = NSPasteboardItem::types(item);
                let mut res = Vec::new();
                for i in 0..NSArray::count(types) {
                    res.push(from_nsstring(NSArray::objectAtIndex(types, i)));
                }
                res
            } else {
                Vec::new()
            }
        });
        Ok(types)
    }

    pub async fn get_data_for_item(
        &self,
        item: i64,
        data_type: String,
    ) -> NativeExtensionsResult<Value> {
        let (future, completer) = FutureCompleter::new();
        let pasteboard = self.pasteboard.clone();
        // Retrieving data may require call back to flutter and nested run loop so don't
        // block current dispatch
        Context::get()
            .run_loop()
            .schedule_next(move || {
                let data = autoreleasepool(|| unsafe {
                    let items: id = msg_send![*pasteboard, pasteboardItems];
                    if item < NSArray::count(items) as i64 {
                        let item = NSArray::objectAtIndex(items, item as NSUInteger);
                        let data_type = to_nsstring(&data_type);
                        // Try to get property list first, otherwise fallback to Data
                        let mut data = NSPasteboardItem::propertyListForType(item, *data_type);
                        if data.is_null() {
                            // Ask for data here. It's better for Appkit to convert String to data,
                            // then trying to convert data to String.
                            data = NSPasteboardItem::dataForType(item, *data_type);
                        }
                        Value::from_objc(data).ok_log().unwrap_or_default()
                    } else {
                        Value::Null
                    }
                });
                completer.complete(data)
            })
            .detach();
        Ok(future.await)
    }

    pub fn new_default() -> NativeExtensionsResult<Self> {
        Ok(Self {
            pasteboard: unsafe { StrongPtr::retain(NSPasteboard::generalPasteboard(nil)) },
        })
    }

    pub fn assign_weak_self(&self, _weak: Weak<PlatformClipboardReader>) {}
}
