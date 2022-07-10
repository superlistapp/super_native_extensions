use std::{
    rc::Weak,
    sync::{Arc, Mutex},
};

use block::ConcreteBlock;
use cocoa::{
    base::id,
    foundation::{NSArray, NSInteger, NSUInteger},
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
    error::NativeExtensionsResult, log::OkLog, platform_impl::platform::common::{from_nsstring, to_nsstring},
};

pub struct PlatformClipboardReader {
    pasteboard: StrongPtr,
}

impl PlatformClipboardReader {
    pub async fn get_items(&self) -> NativeExtensionsResult<Vec<i64>> {
        let count = autoreleasepool(|| unsafe {
            let count: NSInteger = msg_send![*self.pasteboard, numberOfItems];
            count as i64
        });
        Ok((0..count).collect())
    }

    pub async fn get_types_for_item(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        let types = autoreleasepool(|| unsafe {
            let providers: id = msg_send![*self.pasteboard, itemProviders];
            if item < NSArray::count(providers) as i64 {
                let provider = NSArray::objectAtIndex(providers, item as NSUInteger);
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
        Ok(types)
    }

    pub async fn get_data_for_item(
        &self,
        item: i64,
        data_type: String,
    ) -> NativeExtensionsResult<Value> {
        let (future, completer) = FutureCompleter::new();
        autoreleasepool(|| unsafe {
            let providers: id = msg_send![*self.pasteboard, itemProviders];
            if item < NSArray::count(providers) as i64 {
                // travels between threads, must be refcounted because and block is Fn
                let completer = Arc::new(Mutex::new(Capsule::new(completer)));
                let provider = NSArray::objectAtIndex(providers, item as NSUInteger);
                let sender = Context::get().run_loop().new_sender();
                let block = ConcreteBlock::new(move |data: id, _err: id| {
                    struct Movable<T>(T);
                    unsafe impl<T> Send for Movable<T> {}
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
                let () = msg_send![provider, loadDataRepresentationForTypeIdentifier:*to_nsstring(&data_type)
                                      completionHandler:&*block];
            } else {
                completer.complete(Ok(Value::Null));
            }
        });
        future.await
    }

    pub fn new_default() -> NativeExtensionsResult<Self> {
        Ok(Self {
            pasteboard: unsafe {
                StrongPtr::retain(msg_send![class!(UIPasteboard), generalPasteboard])
            },
        })
    }

    pub fn assign_weak_self(&self, _weak: Weak<PlatformClipboardReader>) {}
}
