use std::{
    collections::HashMap,
    mem::ManuallyDrop,
    os::raw::c_void,
    rc::Weak,
    sync::{Arc, Mutex},
};

use block::{Block, RcBlock};
use cocoa::{
    base::{id, nil},
    foundation::NSArray,
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
    api_model::{DataSource, DataSourceItemRepresentation, LazyValueId},
    data_source_manager::PlatformDataSourceDelegate,
    error::NativeExtensionsResult,
    util::DropNotifier,
    value_coerce::{CoerceToData, StringFormat},
    value_promise::ValuePromiseResult,
};

use super::util::{from_nsstring, to_nsstring};

struct State {
    source: DataSource,
    precached_values: HashMap<(LazyValueId, String), ValuePromiseResult>,
}

pub struct PlatformDataSource {
    weak_self: Late<Weak<Self>>,
    delegate: Weak<dyn PlatformDataSourceDelegate>,
    isolate_id: IsolateId,
    state: Arc<Mutex<State>>,
}

impl PlatformDataSource {
    pub fn new(
        delegate: Weak<dyn PlatformDataSourceDelegate>,
        isolate_id: IsolateId,
        source: DataSource,
    ) -> Self {
        Self {
            delegate,
            isolate_id,
            weak_self: Late::new(),
            state: Arc::new(Mutex::new(State {
                source,
                precached_values: HashMap::new(),
            })),
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    pub async fn write_to_clipboard(
        &self,
        drop_notifier: Arc<DropNotifier>,
    ) -> NativeExtensionsResult<()> {
        // iOS general pasteboard is truly braindead. It eagerly fetches all items invoking the
        // provider callbacks on background thread, but it blocks main thread on access until
        // the blocks return value. Which means if we try to schedule anything on main thread
        // it will deadlock. Because iOS prefetches everything anyway, we might as well do it
        // ourselves to avoid the deadlock.
        self.precache().await;
        autoreleasepool(|| unsafe {
            let items = self.create_items(drop_notifier);
            let array = NSArray::arrayWithObjects(nil, &items);
            let pasteboard: id = msg_send![class!(UIPasteboard), generalPasteboard];
            let () = msg_send![pasteboard, setObjects: array];
        });
        Ok(())
    }

    pub fn create_items(&self, drop_notifier: Arc<DropNotifier>) -> Vec<id> {
        let mut items = Vec::<id>::new();
        let state = self.state.clone();
        for item in self.state.lock().unwrap().source.items.iter().enumerate() {
            let sender = Context::get().run_loop().new_sender();
            let state = Arc::new(ItemState {
                source: Capsule::new_with_sender(self.weak_self.clone(), sender.clone()),
                index: item.0,
                sender,
                state: state.clone(),
                _drop_notifier: drop_notifier.clone(),
            });
            let item = state.create_item();
            items.push(item.autorelease());
        }
        items
    }

    async fn precache(&self) {
        let to_fetch = {
            let state = self.state.lock().unwrap();
            let mut items = Vec::<(LazyValueId, String)>::new();
            for item in &state.source.items {
                for data in &item.representations {
                    match data {
                        DataSourceItemRepresentation::Lazy { formats, id } => {
                            for format in formats {
                                let key = (*id, format.clone());
                                if !state.precached_values.contains_key(&key) {
                                    items.push(key);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            items
        };

        if let Some(delegate) = self.delegate.upgrade() {
            for item in to_fetch {
                let res = delegate
                    .get_lazy_data_async(self.isolate_id, item.0, item.1.clone())
                    .await;
                let mut state = self.state.lock().unwrap();
                state.precached_values.insert(item, res);
            }
        }
    }
}

struct ItemState {
    source: Capsule<Weak<PlatformDataSource>>,
    index: usize,
    sender: RunLoopSender,
    state: Arc<Mutex<State>>,
    _drop_notifier: Arc<DropNotifier>,
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
        let item = &state.source.items[self.index];
        let types: Vec<_> = item
            .representations
            .iter()
            .filter_map(|d| match d {
                DataSourceItemRepresentation::Simple { formats, data: _ } => Some(
                    formats
                        .iter()
                        .map(|t| to_nsstring(t).autorelease())
                        .collect::<Vec<_>>(),
                ),
                DataSourceItemRepresentation::Lazy { formats, id: _ } => Some(
                    formats
                        .iter()
                        .map(|t| to_nsstring(t).autorelease())
                        .collect::<Vec<_>>() as Vec<_>,
                ),
                DataSourceItemRepresentation::VirtualFile {
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

    fn fetch_value(&self, id: LazyValueId, format: String, handler: RcBlock<(id, id), ()>) {
        let source = self.source.clone();
        let handler = Movable(handler);
        self.sender.send(move || {
            let handler = handler;
            if let Some(source) = source.get_ref().ok().and_then(|c| c.upgrade()) {
                Context::get().run_loop().spawn(async move {
                    if let Some(delegate) = source.delegate.upgrade() {
                        let data = delegate
                            .get_lazy_data_async(source.isolate_id, id, format)
                            .await;
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

        let format = unsafe { from_nsstring(pasteboard_type) };
        let item = &state.source.items[self.index];
        for data in &item.representations {
            match data {
                DataSourceItemRepresentation::Simple { formats, data } => {
                    if formats.contains(&format) {
                        unsafe { handler.call((Self::value_to_nsdata(data), nil)) };
                        return nil;
                    }
                }
                DataSourceItemRepresentation::Lazy { formats, id } => {
                    if formats.contains(&format) {
                        let precached = state.precached_values.get(&(*id, format.clone()));
                        match precached {
                            Some(value) => unsafe {
                                handler.call((Self::value_promise_res_to_nsdata(value), nil));
                            },
                            None => {
                                self.fetch_value(*id, format, handler);
                            }
                        }
                        return nil;
                    }
                }
                _ => {}
            }
        }
        nil
    }
}

fn item_state(this: &Object) -> Option<Arc<ItemState>> {
    unsafe {
        let state_ptr = {
            let state_ptr: *mut c_void = *this.get_ivar("imState");
            state_ptr as *const ItemState
        };
        if state_ptr.is_null() {
            None
        } else {
            let state = Arc::from_raw(state_ptr);
            let res = state.clone();
            let _ = ManuallyDrop::new(state);
            Some(res)
        }
    }
}

extern "C" fn writable_types_for_item_provider(this: &Object, _sel: Sel) -> id {
    if let Some(state) = item_state(this) {
        state.writable_types()
    } else {
        nil
    }
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
    if let Some(state) = item_state(this) {
        let handler = unsafe { &mut *(handler as *mut Block<(id, id), ()>) };
        let handler = unsafe { RcBlock::copy(handler as *mut _) };
        state.data_for_type(identifier, handler)
    } else {
        nil
    }
}

extern "C" fn dispose_state(this: &mut Object, _sel: Sel) {
    unsafe {
        let state_ptr = {
            let state_ptr: *mut c_void = *this.get_ivar("imState");
            state_ptr as *const ItemState
        };
        if !state_ptr.is_null() {
            Arc::from_raw(state_ptr);
            this.set_ivar("imState", std::ptr::null_mut() as *mut c_void);
        }
    }
}

extern "C" fn dealloc(this: &Object, _sel: Sel) {
    unsafe {
        let _: () = msg_send![this, disposeState];

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
    decl.add_method(
        sel!(disposeState),
        dispose_state as extern "C" fn(&mut Object, Sel),
    );
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
