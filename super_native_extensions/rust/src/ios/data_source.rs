use std::{
    collections::HashMap,
    rc::Weak,
    sync::{Arc, Mutex},
};

use block::{Block, ConcreteBlock, RcBlock};
use cocoa::{
    base::{id, nil},
    foundation::{NSArray, NSUInteger},
};
use nativeshell_core::{
    util::{Capsule, Late},
    Context, IsolateId, RunLoopSender, Value,
};
use objc::{
    class, msg_send,
    rc::{autoreleasepool, StrongPtr},
    sel, sel_impl,
};

use crate::{
    api_model::{DataSource, DataSourceItemRepresentation, DataSourceValueId},
    data_source_manager::PlatformDataSourceDelegate,
    error::NativeExtensionsResult,
    util::DropNotifier,
    value_coerce::{CoerceToData, StringFormat},
    value_promise::ValuePromiseResult,
};

use super::util::to_nsstring;

struct State {
    source: DataSource,
    precached_values: HashMap<(DataSourceValueId, String), ValuePromiseResult>,
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

    pub fn create_items(
        &self,
        drop_notifier: Arc<DropNotifier>,
        delegate: Weak<dyn SessionDelegate>,
    ) -> (Vec<id>, Arc<Session>) {
        let mut items = Vec::<id>::new();
        let session = Session::new(
            self.state.clone(),
            drop_notifier,
            self.weak_self.clone(),
            delegate,
        );

        let state = self.state.lock().unwrap();
        for (index, item) in state.source.items.iter().enumerate() {
            unsafe {
                let item_provider: id = msg_send![class!(NSItemProvider), new];
                let item_provider: id = msg_send![item_provider, autorelease];
                if let Some(name) = &item.suggested_name {
                    let name = to_nsstring(name);
                    let () = msg_send![item_provider, setSuggestedName:*name];
                }
                for representation in &item.representations {
                    let formats = match representation {
                        DataSourceItemRepresentation::Simple { formats, data: _ } => Some(formats),
                        DataSourceItemRepresentation::Lazy { formats, id: _ } => Some(formats),
                        _ => None,
                    };
                    if let Some(formats) = formats {
                        for format in formats {
                            let session_clone = session.clone();
                            let format_clone = format.clone();
                            register_data_representation(item_provider, &format, move |callback| {
                                session_clone.value_for_index(index, &format_clone, callback)
                            });
                        }
                    }
                }

                items.push(item_provider);
            }
        }
        (items, session)
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
            let (items, _) = self.create_items(drop_notifier, self.weak_self.clone());
            let array = NSArray::arrayWithObjects(nil, &items);
            let pasteboard: id = msg_send![class!(UIPasteboard), generalPasteboard];
            let () = msg_send![pasteboard, setItemProviders: array];
        });
        Ok(())
    }

    async fn precache(&self) {
        let to_fetch = {
            let state = self.state.lock().unwrap();
            let mut items = Vec::<(DataSourceValueId, String)>::new();
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

impl SessionDelegate for PlatformDataSource {
    fn should_fetch_items(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct Movable<T>(T);

unsafe impl<T> Send for Movable<T> {}

fn register_data_representation<F>(item_provider: id, type_identifier: &str, handler: F)
where
    F: Fn(Box<dyn Fn(id, id) + 'static + Send>) -> id + 'static + Send,
{
    let handler = Box::new(handler);
    let block = ConcreteBlock::new(move |completion_block: id| -> id {
        let completion_block = unsafe { &mut *(completion_block as *mut Block<(id, id), ()>) };
        let completion_block = unsafe { RcBlock::copy(completion_block) };
        let completion_block = Movable(completion_block);
        let completion_fn = move |data: id, err: id| {
            let completion_block = completion_block.clone();
            unsafe { completion_block.0.call((data, err)) };
        };
        handler(Box::new(completion_fn))
    });
    let block = block.copy();
    let type_identifier = to_nsstring(type_identifier);
    unsafe {
        let () = msg_send![item_provider,
            registerDataRepresentationForTypeIdentifier:*type_identifier
            visibility: 0 as NSUInteger // all
            loadHandler: &*block];
    }
}

fn value_to_nsdata(value: &Value) -> StrongPtr {
    let buf = value.coerce_to_data(StringFormat::Utf8);
    match buf {
        Some(data) => to_nsdata(&data),
        None => unsafe { StrongPtr::new(std::ptr::null_mut()) },
    }
}

fn value_promise_res_to_nsdata(value: &ValuePromiseResult) -> StrongPtr {
    match value {
        ValuePromiseResult::Ok { value } => value_to_nsdata(value),
        ValuePromiseResult::Cancelled => unsafe { StrongPtr::new(std::ptr::null_mut()) },
    }
}

pub fn to_nsdata(data: &[u8]) -> StrongPtr {
    unsafe {
        let d: id = msg_send![class!(NSData), alloc];
        let d: id = msg_send![d, initWithBytes:data.as_ptr() length:data.len()];
        StrongPtr::new(d)
    }
}

pub trait SessionDelegate {
    fn should_fetch_items(&self) -> bool;
}

struct SessionInner {
    state: Arc<Mutex<State>>,
    _drop_notifier: Arc<DropNotifier>,
    sender: RunLoopSender,
    platform_source: Mutex<Capsule<Weak<PlatformDataSource>>>,
    delegate: Mutex<Capsule<Weak<dyn SessionDelegate>>>,
}

impl SessionInner {
    fn fetch_value(
        &self,
        id: DataSourceValueId,
        format: String,
        callback: Box<dyn Fn(id, id) + Send>,
    ) -> id {
        let platform_source = self.platform_source.lock().unwrap().clone();
        let session_delegate = self.delegate.lock().unwrap().clone();
        self.sender.send(move || {
            if let Some(session_delegate) =
                session_delegate.get_ref().ok().and_then(|s| s.upgrade())
            {
                // For some reason iOS seems to eagerly fetch items immediatelly
                // at the beginning of drag (before even  dragInteraction:sessionWillBegin:).
                // If we detect that return empty data.
                if !session_delegate.should_fetch_items() {
                    callback(nil, nil);
                    return;
                }
            }
            println!("Fetch data!!!");
            if let Some(source) = platform_source.get_ref().ok().and_then(|c| c.upgrade()) {
                if let Some(delegate) = source.delegate.upgrade() {
                    Context::get().run_loop().spawn(async move {
                        let data = delegate
                            .get_lazy_data_async(source.isolate_id, id, format)
                            .await;
                        callback(*value_promise_res_to_nsdata(&data), nil);
                    });
                } else {
                    callback(nil, nil);
                }
            }
        });
        nil // NSProgress
    }

    fn value_for_index(
        self: &Arc<Self>,
        index: usize,
        format: &String,
        callback: Box<dyn Fn(id, id) + Send>,
    ) -> id {
        let state = self.state.lock().unwrap();
        let item = &state.source.items[index];
        for representation in &item.representations {
            match representation {
                DataSourceItemRepresentation::Simple { formats, data } => {
                    if formats.contains(format) {
                        let data = value_to_nsdata(data);
                        callback(*data, nil);
                        return nil;
                    }
                }
                DataSourceItemRepresentation::Lazy { formats, id } => {
                    if formats.contains(format) {
                        let precached = state.precached_values.get(&(*id, format.clone()));
                        match precached {
                            Some(value) => {
                                let data = value_promise_res_to_nsdata(value);
                                callback(*data, nil);
                                return nil;
                            }
                            None => return self.fetch_value(*id, format.clone(), callback),
                        }
                    }
                }
                _ => {}
            }
        }
        callback(nil, nil);
        nil // NSProgress
    }
}

pub struct Session {
    inner: Mutex<Option<Arc<SessionInner>>>,
}

impl Session {
    fn new(
        state: Arc<Mutex<State>>,
        drop_notifier: Arc<DropNotifier>,
        platform_source: Weak<PlatformDataSource>,
        delegate: Weak<dyn SessionDelegate>,
    ) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(Some(Arc::new(SessionInner {
                state,
                _drop_notifier: drop_notifier,
                sender: Context::get().run_loop().new_sender(),
                platform_source: Mutex::new(Capsule::new(platform_source)),
                delegate: Mutex::new(Capsule::new(delegate)),
            }))),
        })
    }

    fn value_for_index(
        self: &Arc<Self>,
        index: usize,
        format: &String,
        callback: Box<dyn Fn(id, id) + Send>,
    ) -> id {
        let inner = self.inner.lock().unwrap();
        match &*inner {
            Some(inner) => inner.value_for_index(index, format, callback),
            None => {
                callback(nil, nil);
                nil
            }
        }
    }

    pub fn dispose(self: &Arc<Self>) {
        self.inner.lock().unwrap().take();
    }
}
