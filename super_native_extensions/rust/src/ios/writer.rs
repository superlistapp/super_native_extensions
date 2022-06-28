use std::{
    collections::HashMap,
    ffi::c_void,
    mem::ManuallyDrop,
    os::raw::c_char,
    rc::Weak,
    slice,
    sync::{Arc, Mutex},
};

use block::{Block, RcBlock};
use cocoa::{
    base::{id, nil},
    foundation::{NSArray, NSString},
};
use nativeshell_core::{
    util::{Capsule, Late},
    Context, IsolateId, RunLoopSender, Value,
};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    rc::{autoreleasepool, StrongPtr},
    runtime::{Class, Object, Protocol, Sel},
    sel, sel_impl,
};
use once_cell::sync::Lazy;

use crate::{
    error::ClipboardResult,
    value_coerce::{CoerceToData, StringFormat},
    value_promise::ValuePromiseResult,
    writer_data::{ClipboardWriterData, ClipboardWriterItemData},
    writer_manager::PlatformClipboardWriterDelegate,
};

struct State {
    data: ClipboardWriterData,
    precached_values: HashMap<i64, ValuePromiseResult>,
}

pub struct PlatformClipboardWriter {
    weak_self: Late<Weak<Self>>,
    delegate: Weak<dyn PlatformClipboardWriterDelegate>,
    isolate_id: IsolateId,
    state: Arc<Mutex<State>>,
}

impl PlatformClipboardWriter {
    pub fn new(
        delegate: Weak<dyn PlatformClipboardWriterDelegate>,
        isolate_id: IsolateId,
        data: ClipboardWriterData,
    ) -> Self {
        Self {
            delegate,
            isolate_id,
            weak_self: Late::new(),
            state: Arc::new(Mutex::new(State {
                data,
                precached_values: HashMap::new(),
            })),
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    pub fn create_items(&self) -> Vec<id> {
        let mut items = Vec::<id>::new();
        let state = self.state.clone();
        for item in self.state.lock().unwrap().data.items.iter().enumerate() {
            let sender = Context::get().run_loop().new_sender();
            let state = Arc::new(ItemState {
                clipboard: Capsule::new_with_sender(self.weak_self.clone(), sender.clone()),
                index: item.0,
                sender,
                state: state.clone(),
            });
            let item = state.create_item();
            items.push(item.autorelease());
        }
        items
    }

    async fn write_to_pasteboard(&self, pasteboard: id) -> ClipboardResult<()> {
        autoreleasepool(|| unsafe {
            let items = self.create_items();
            let array = NSArray::arrayWithObjects(nil, &items);
            let () = msg_send![pasteboard, setObjects: array];
        });
        Ok(())
    }

    async fn precache(&self) {
        let to_fetch = {
            let state = self.state.lock().unwrap();
            let mut items = Vec::<i64>::new();
            for item in &state.data.items {
                for data in &item.data {
                    match data {
                        ClipboardWriterItemData::Lazy { types: _, id } => {
                            if !state.precached_values.contains_key(id) {
                                items.push(*id);
                            }
                        }
                        _ => {}
                    }
                }
            }
            items
        };

        if let Some(delegate) = self.delegate.upgrade() {
            for id in to_fetch {
                let res = delegate.get_lazy_data_async(self.isolate_id, id).await;
                let mut state = self.state.lock().unwrap();
                state.precached_values.insert(id, res);
            }
        }
    }

    pub async fn write_to_clipboard(&self) -> ClipboardResult<()> {
        // iOS general pasteboard is truly braindead. It eagerly fetches all items invoking the
        // provider callbacks on background thread, but it blocks main thread on access until
        // the blocks return value. Which means if we try to schedule anything on main thread
        // it will deadlock. Because iOS prefetches everything anyway, we might as well do it
        // ourselves to avoid the deadlock.
        self.precache().await;

        let pasteboard =
            unsafe { StrongPtr::retain(msg_send![class!(UIPasteboard), generalPasteboard]) };
        self.write_to_pasteboard(*pasteboard).await
    }
}

pub fn to_nsstring(string: &str) -> StrongPtr {
    unsafe {
        let ptr = NSString::alloc(nil).init_str(string);
        StrongPtr::new(ptr)
    }
}

const UTF8_ENCODING: usize = 4;

pub unsafe fn from_nsstring(ns_string: id) -> String {
    let bytes: *const c_char = msg_send![ns_string, UTF8String];
    let bytes = bytes as *const u8;

    let len = msg_send![ns_string, lengthOfBytesUsingEncoding: UTF8_ENCODING];

    let bytes = slice::from_raw_parts(bytes, len);
    std::str::from_utf8(bytes).unwrap().into()
}

struct ItemState {
    clipboard: Capsule<Weak<PlatformClipboardWriter>>,
    index: usize,
    sender: RunLoopSender,
    state: Arc<Mutex<State>>,
}

impl ItemState {
    fn create_item(self: Arc<Self>) -> StrongPtr {
        unsafe {
            let item: id = msg_send![*PASTEBOARD_WRITER_CLASS, alloc];
            let () = msg_send![item, init];
            (*item).set_ivar("imState", Arc::into_raw(self) as *mut c_void);
            StrongPtr::new(item)
        }
    }

    fn writable_types(&self) -> id {
        let state = self.state.lock().unwrap();
        let item = &state.data.items[self.index];
        let types: Vec<_> = item
            .data
            .iter()
            .filter_map(|d| match d {
                crate::writer_data::ClipboardWriterItemData::Simple { types, data: _ } => Some(
                    types
                        .iter()
                        .map(|t| to_nsstring(t).autorelease())
                        .collect::<Vec<_>>(),
                ),
                crate::writer_data::ClipboardWriterItemData::Lazy { types, id: _ } => Some(
                    types
                        .iter()
                        .map(|t| to_nsstring(t).autorelease())
                        .collect::<Vec<_>>() as Vec<_>,
                ),
                crate::writer_data::ClipboardWriterItemData::VirtualFile {
                    file_size: _,
                    file_name: _,
                } => None,
            })
            .flatten()
            .collect();
        unsafe { NSArray::arrayWithObjects(nil, &types) }
    }

    fn value_to_nsdata(value: &Value) -> id {
        let buf = value.coerce_to_data(StringFormat::Utf8);
        match buf {
            Some(data) => to_nsdata(&data).autorelease(),
            None => nil,
        }
    }

    fn value_promise_res_to_nsdata(value: &ValuePromiseResult) -> id {
        match value {
            ValuePromiseResult::Ok { value } => Self::value_to_nsdata(value),
            ValuePromiseResult::Cancelled => nil,
        }
    }

    fn fetch_value(&self, id: i64, handler: RcBlock<(id, id), ()>) {
        let clipboard = self.clipboard.clone();
        let handler = Movable(handler);
        self.sender.send(move || {
            let handler = handler;
            if let Some(clipboard) = clipboard.get_ref().ok().and_then(|c| c.upgrade()) {
                Context::get().run_loop().spawn(async move {
                    if let Some(delegate) = clipboard.delegate.upgrade() {
                        let data = delegate.get_lazy_data_async(clipboard.isolate_id, id).await;
                        unsafe {
                            handler
                                .0
                                .call((Self::value_promise_res_to_nsdata(&data), nil))
                        };
                    } else {
                        unsafe { handler.0.call((nil, nil)) };
                    }
                });
            } else {
                unsafe { handler.0.call((nil, nil)) };
            }
        });
    }

    fn data_for_type(&self, pasteboard_type: id, handler: RcBlock<(id, id), ()>) -> id {
        let state = self.state.lock().unwrap();

        let ty = unsafe { from_nsstring(pasteboard_type) };
        let item = &state.data.items[self.index];
        for data in &item.data {
            match data {
                crate::writer_data::ClipboardWriterItemData::Simple { types, data } => {
                    if types.contains(&ty) {
                        unsafe { handler.call((Self::value_to_nsdata(data), nil)) };
                        return nil;
                    }
                }
                crate::writer_data::ClipboardWriterItemData::Lazy { types, id } => {
                    if types.contains(&ty) {
                        let precached = state.precached_values.get(id);
                        match precached {
                            Some(value) => unsafe {
                                handler.call((Self::value_promise_res_to_nsdata(value), nil));
                            },
                            None => {
                                self.fetch_value(*id, handler);
                            }
                        }
                        return nil;
                    }
                }
                crate::writer_data::ClipboardWriterItemData::VirtualFile {
                    file_size: _,
                    file_name: _,
                } => {}
            }
        }
        nil
    }
}

fn item_state(this: &Object) -> Arc<ItemState> {
    unsafe {
        let state_ptr = {
            let state_ptr: *mut c_void = *this.get_ivar("imState");
            state_ptr as *const ItemState
        };
        let ptr = Arc::from_raw(state_ptr);
        let res = ptr.clone();
        let _ = ManuallyDrop::new(ptr);
        res
    }
}

extern "C" fn writable_types_for_item_provider(this: &Object, _sel: Sel) -> id {
    let state = item_state(this);
    state.writable_types()
}

extern "C" fn writable_types_for_item_provider_(_this: &Class, _sel: Sel) -> id {
    // Class method - we're not interested in this
    unsafe { NSArray::arrayWithObjects(nil, &[]) }
}

struct Movable<T>(T);

unsafe impl<T> Send for Movable<T> {}

extern "C" fn load_data_with_type_identifier(
    this: &Object,
    _sel: Sel,
    identifier: id,
    handler: id,
) -> id {
    let handler = unsafe { &mut *(handler as *mut Block<(id, id), ()>) };
    let handler = unsafe { RcBlock::copy(handler as *mut _) };
    let state = item_state(this);
    state.data_for_type(identifier, handler)
}

extern "C" fn dealloc(this: &Object, _sel: Sel) {
    unsafe {
        let state_ptr = {
            let state_ptr: *mut c_void = *this.get_ivar("imState");
            state_ptr as *const ItemState
        };
        Arc::from_raw(state_ptr);

        let superclass = superclass(this);
        let () = msg_send![super(this, superclass), dealloc];
    }
}

static PASTEBOARD_WRITER_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("IMItemDataProviderWriter", superclass).unwrap();
    decl.add_ivar::<*mut c_void>("imState");
    if let Some(protocol) = Protocol::get("NSItemProviderWriting") {
        decl.add_protocol(protocol);
    }
    decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
    decl.add_class_method(
        sel!(writable_types_for_item_provider),
        writable_types_for_item_provider_ as extern "C" fn(&Class, Sel) -> id,
    );
    decl.add_method(
        sel!(writableTypeIdentifiersForItemProvider),
        writable_types_for_item_provider as extern "C" fn(&Object, Sel) -> id,
    );
    decl.add_method(
        sel!(loadDataWithTypeIdentifier:forItemProviderCompletionHandler:),
        load_data_with_type_identifier as extern "C" fn(&Object, Sel, id, id) -> id,
    );

    decl.register()
});

pub unsafe fn superclass(this: &Object) -> &Class {
    let superclass: id = msg_send![this, superclass];
    &*(superclass as *const _)
}

pub fn to_nsdata(data: &[u8]) -> StrongPtr {
    unsafe {
        let d: id = msg_send![class!(NSData), alloc];
        let d: id = msg_send![d, initWithBytes:data.as_ptr() length:data.len()];
        StrongPtr::new(d)
    }
}
