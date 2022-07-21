use std::{
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
};

use block::ConcreteBlock;
use cocoa::{
    base::{id, nil},
    foundation::NSArray,
};

use nativeshell_core::{
    platform::value::ValueObjcConversion,
    util::{Capsule, FutureCompleter},
    Context, Value,
};
use objc::{
    class, msg_send,
    rc::{autoreleasepool, StrongPtr},
    sel, sel_impl,
};

use crate::{
    error::NativeExtensionsResult,
    log::OkLog,
    platform_impl::platform::common::{from_nsstring, to_nsstring},
};

pub struct PlatformDataReader {
    source: ReaderSource,
}

enum ReaderSource {
    Pasteboard(StrongPtr),
    DropSessionItems(StrongPtr),
}

impl PlatformDataReader {
    fn get_items_providers(&self) -> Vec<id> {
        let mut res = Vec::<id>::new();
        match &self.source {
            ReaderSource::Pasteboard(pasteboard) => {
                let providers: id = unsafe { msg_send![**pasteboard, itemProviders] };
                for i in 0..unsafe { NSArray::count(providers) } {
                    let provider = unsafe { NSArray::objectAtIndex(providers, i) };
                    res.push(provider);
                }
            }
            ReaderSource::DropSessionItems(items) => {
                for i in 0..unsafe { NSArray::count(**items) } {
                    let item: id = unsafe { NSArray::objectAtIndex(**items, i) };
                    let provider: id = unsafe { msg_send![item, itemProvider] };
                    res.push(provider);
                }
            }
        }
        res
    }

    pub async fn get_items(&self) -> NativeExtensionsResult<Vec<i64>> {
        let count = autoreleasepool(|| {
            let providers = self.get_items_providers();
            providers.len() as i64
        });
        Ok((0..count).collect())
    }

    pub async fn get_formats_for_item(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        let formats = autoreleasepool(|| unsafe {
            let providers = self.get_items_providers();
            if item < providers.len() as i64 {
                let provider = providers[item as usize];
                let identifiers: id = msg_send![provider, registeredTypeIdentifiers];
                let mut res = Vec::new();
                for i in 0..NSArray::count(identifiers) {
                    res.push(from_nsstring(NSArray::objectAtIndex(identifiers, i)));
                }
                res
            } else {
                Vec::new()
            }
        });
        Ok(formats)
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
    ) -> NativeExtensionsResult<Value> {
        let (future, completer) = FutureCompleter::new();
        autoreleasepool(|| unsafe {
            let providers = self.get_items_providers();
            if item < providers.len() as i64 {
                // travels between threads, must be refcounted because and block is Fn
                let completer = Arc::new(Mutex::new(Capsule::new(completer)));
                let provider = providers[item as usize];
                let sender = Context::get().run_loop().new_sender();
                let block = ConcreteBlock::new(move |data: id, _err: id| {
                    struct Movable<T>(T);
                    unsafe impl<T> Send for Movable<T> {}
                    let data = Self::maybe_decode_bplist(data);
                    let data = Movable(StrongPtr::retain(data));
                    let completer = completer.clone();
                    sender.send(move || {
                        let completer = completer
                            .lock()
                            .unwrap()
                            .take()
                            .expect("Block invoked more than once");
                        let data = data;
                        completer
                            .complete(Ok(Value::from_objc(*data.0).ok_log().unwrap_or_default()))
                    });
                });
                let block = block.copy();
                let () = msg_send![provider, loadDataRepresentationForTypeIdentifier:*to_nsstring(&format)
                                      completionHandler:&*block];
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

    pub fn assign_weak_self(&self, _weak: Weak<PlatformDataReader>) {}
}
