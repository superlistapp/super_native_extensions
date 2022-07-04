use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::{CStr, OsStr},
    fs::File,
    io::Write,
    mem::ManuallyDrop,
    os::{
        raw::c_void,
        unix::prelude::{FromRawFd, IntoRawFd, OsStrExt},
    },
    path::PathBuf,
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
};

use block::{Block, ConcreteBlock, RcBlock};
use cocoa::{
    appkit::NSPasteboard,
    base::{id, nil, YES},
    foundation::{NSArray, NSUInteger},
};
use core_foundation::{runloop::CFRunLoopRunInMode, string::CFStringRef};
use nativeshell_core::{platform::value::ValueObjcConversion, util::Late, IsolateId};
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
    api_model::{DataSource, DataSourceItemRepresentation, DataSourceValueId},
    data_source_manager::{PlatformDataSourceDelegate, VirtualFileResult},
    error::NativeExtensionsResult,
    log::OkLog,
    util::DropNotifier,
    value_promise::ValuePromiseResult,
};

use super::util::{from_nsstring, superclass, to_nserror, to_nsstring};

pub fn platform_stream_write(handle: i32, data: &[u8]) -> i32 {
    let mut file = ManuallyDrop::new(unsafe { File::from_raw_fd(handle) });
    match file.write_all(data) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

static FILE_PATHS: Lazy<Mutex<HashMap<i32, PathBuf>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub fn platform_stream_close(handle: i32, delete: bool) {
    unsafe { File::from_raw_fd(handle) };
    let path = FILE_PATHS.lock().unwrap().remove(&handle);
    if let Some(path) = path {
        if delete {
            std::fs::remove_file(&path).ok();
        }
    }
}

pub struct PlatformDataSource {
    weak_self: Late<Weak<Self>>,
    delegate: Weak<dyn PlatformDataSourceDelegate>,
    isolate_id: IsolateId,
    data: DataSource,
}

impl PlatformDataSource {
    pub fn new(
        delegate: Weak<dyn PlatformDataSourceDelegate>,
        isolate_id: IsolateId,
        data: DataSource,
    ) -> Self {
        Self {
            delegate,
            data,
            isolate_id,
            weak_self: Late::new(),
        }
    }
    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    pub fn create_items(&self, drop_notifier: Arc<DropNotifier>) -> Vec<id> {
        let mut items = Vec::<id>::new();
        for item in self.data.items.iter().enumerate() {
            let state = Rc::new(ItemState {
                data_source: self.weak_self.clone(),
                index: item.0,
                _drop_notifier: drop_notifier.clone(),
            });
            let item = state.create_item();
            items.push(item.autorelease());
        }
        items
    }

    pub async fn write_to_clipboard(
        &self,
        drop_notifier: Arc<DropNotifier>,
    ) -> NativeExtensionsResult<()> {
        autoreleasepool(|| unsafe {
            let items = self.create_items(drop_notifier);
            let array = NSArray::arrayWithObjects(nil, &items);
            let pasteboard = NSPasteboard::generalPasteboard(nil);
            NSPasteboard::clearContents(pasteboard);
            NSPasteboard::writeObjects(pasteboard, array);
        });
        Ok(())
    }
}

struct ItemState {
    data_source: Weak<PlatformDataSource>,
    index: usize,
    _drop_notifier: Arc<DropNotifier>,
}

struct VirtualFileInfo {
    id: DataSourceValueId,
    format: String,
}

impl ItemState {
    fn create_item(self: Rc<Self>) -> StrongPtr {
        let item = unsafe {
            let item: id = msg_send![*PASTEBOARD_WRITER_CLASS, new];
            (*item).set_ivar("sneState", Rc::into_raw(self.clone()) as *mut c_void);
            StrongPtr::new(item)
        };

        let info = self.virtual_file_info();
        match info {
            Some(info) => {
                unsafe {
                    let provider: id = msg_send![class!(SNEForwardingFilePromiseProvider), new];
                    let () = msg_send![provider, setFileType: *to_nsstring(&info.format)];
                    let () = msg_send![provider, setDelegate: *item];
                    let () = msg_send![provider, setWritingDelegate: *item]; // this one is strong reference
                    StrongPtr::new(provider)
                }
            }
            None => item,
        }
    }

    fn virtual_file_info(self: &Rc<Self>) -> Option<VirtualFileInfo> {
        if Class::get("NSFilePromiseProvider").is_none() {
            return None;
        }
        self.data_source.upgrade().and_then(|data_source| {
            let item = &data_source.data.items[self.index];
            item.representations.iter().find_map(|item| match item {
                DataSourceItemRepresentation::VirtualFile {
                    id,
                    file_size: _,
                    format,
                    storage_suggestion: _,
                } => Some(VirtualFileInfo {
                    id: *id,
                    format: format.clone(),
                }),
                _ => None,
            })
        })
    }

    fn writable_types(&self) -> id {
        match self.data_source.upgrade() {
            Some(data_source) => {
                let item = &data_source.data.items[self.index];
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
                        _ => None,
                    })
                    .flatten()
                    .collect();
                unsafe { NSArray::arrayWithObjects(nil, &types) }
            }
            None => nil,
        }
    }

    fn object_for_type(&self, pasteboard_type: id) -> id {
        match self.data_source.upgrade() {
            Some(data_source) => {
                let ty = unsafe { from_nsstring(pasteboard_type) };
                let item = &data_source.data.items[self.index];
                for data in &item.representations {
                    match data {
                        DataSourceItemRepresentation::Simple { formats, data } => {
                            if formats.contains(&ty) {
                                return data
                                    .to_objc()
                                    .ok_log()
                                    .map(|a| a.autorelease())
                                    .unwrap_or(nil);
                            }
                        }
                        DataSourceItemRepresentation::Lazy { formats, id } => {
                            if formats.contains(&ty) {
                                if let Some(delegate) = data_source.delegate.upgrade() {
                                    let promise = delegate.get_lazy_data(
                                        data_source.isolate_id,
                                        *id,
                                        ty,
                                        None,
                                    );
                                    loop {
                                        if let Some(result) = promise.try_take() {
                                            match result {
                                                ValuePromiseResult::Ok { value } => {
                                                    return value
                                                        .to_objc()
                                                        .ok_log()
                                                        .map(|a| a.autorelease())
                                                        .unwrap_or(nil);
                                                }
                                                ValuePromiseResult::Cancelled => {
                                                    return nil;
                                                }
                                            }
                                        }
                                        let mode = to_nsstring("NativeShellRunLoopMode");
                                        unsafe { CFRunLoopRunInMode(*mode as CFStringRef, 1.0, 1) };
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                nil
            }
            None => nil,
        }
    }

    fn file_promise_file_name_for_type(self: &Rc<Self>, _file_type: id) -> id {
        match self.data_source.upgrade() {
            Some(data_source) => {
                let item = &data_source.data.items[self.index];
                item.suggested_name
                    .as_ref()
                    .map(|name| to_nsstring(&name).autorelease())
                    .unwrap_or(nil)
            }
            None => nil,
        }
    }

    fn progress_for_url(url: id) -> StrongPtr {
        unsafe {
            let progress = StrongPtr::retain(
                msg_send![class!(NSProgress), progressWithTotalUnitCount: 100 as u64],
            );
            let () = msg_send![*progress, setKind:*to_nsstring("NSProgressKindFile")];
            let () = msg_send![*progress, setFileURL: url];
            let () = msg_send![*progress, setCancellable: YES];
            let () = msg_send![*progress, publish];
            progress
        }
    }

    fn path_from_url(url: id) -> PathBuf {
        let path: *const i8 = unsafe { msg_send![url, fileSystemRepresentation] };
        let path = unsafe { CStr::from_ptr(path) };
        let path = OsStr::from_bytes(path.to_bytes());
        path.into()
    }

    fn file_promise_do_write(
        url: id,
        completion_fn: Box<dyn FnOnce(id)>,
        info: VirtualFileInfo,
        data_source: Rc<PlatformDataSource>,
        delegate: Rc<dyn PlatformDataSourceDelegate>,
    ) {
        let progress = Self::progress_for_url(url);

        let path = Self::path_from_url(url);
        let file = File::create(&path);
        let file = match file {
            Ok(file) => file,
            Err(err) => {
                let error = to_nserror("super_dnd", 0, &err.to_string());
                completion_fn(*error);
                return;
            }
        };
        let descriptor = file.into_raw_fd();
        FILE_PATHS.lock().unwrap().insert(descriptor, path.clone());

        // Keep the notifier alive until the block is alive.
        let notifier_holder = Rc::new(RefCell::new(Option::<Arc<DropNotifier>>::None));
        let notifier_holder_clone = notifier_holder.clone();
        let progress_clone = progress.clone();
        let progress_clone2 = progress.clone();
        let notifier = delegate.get_virtual_file(
            data_source.isolate_id,
            info.id,
            descriptor,
            Box::new(move |cnt| {
                let () = unsafe { msg_send![*progress_clone, setCompletedUnitCount: cnt as u64] };
            }),
            Box::new(move |result| {
                let _notifier = notifier_holder_clone;
                unsafe {
                    let () = msg_send![*progress_clone2, unpublish];
                }
                match result {
                    VirtualFileResult::Done => completion_fn(nil),
                    VirtualFileResult::Error { message } => {
                        let error = to_nserror("super_dnd", 0, &message);
                        completion_fn(*error);
                    }
                    VirtualFileResult::Cancelled => completion_fn(nil),
                }
            }),
        );
        let notifier_clone = notifier.clone();
        let progress_clone = progress.clone();
        let cancellation_handler = ConcreteBlock::new(move || {
            unsafe {
                let () = msg_send![*progress_clone, unpublish];
            }
            notifier_clone.dispose();
        });
        let cancellation_handler = cancellation_handler.copy();
        unsafe {
            let () = msg_send![*progress, setCancellationHandler:&*cancellation_handler];
        }
        notifier_holder.borrow_mut().replace(notifier);
    }

    fn file_promise_write_to_url(self: &Rc<Self>, url: id, completion_fn: Box<dyn FnOnce(id)>) {
        let info = self.virtual_file_info();
        let data_source = self.data_source.upgrade();
        let delegate = data_source.as_ref().and_then(|c| c.delegate.upgrade());

        match (info, data_source, delegate) {
            (Some(info), Some(clipboard), Some(delegate)) => {
                Self::file_promise_do_write(url, completion_fn, info, clipboard, delegate);
            }
            _ => {
                let error = to_nserror("super_dnd", 0, "data not found");
                completion_fn(*error);
            }
        }
    }
}

fn item_state(this: &Object) -> Rc<ItemState> {
    unsafe {
        let state_ptr = {
            let state_ptr: *mut c_void = *this.get_ivar("sneState");
            state_ptr as *const ItemState
        };
        let ptr = Rc::from_raw(state_ptr);
        let res = ptr.clone();
        let _ = ManuallyDrop::new(ptr);
        res
    }
}

extern "C" fn writable_types_for_pasteboard(this: &Object, _sel: Sel, _pasteboard: id) -> id {
    let state = item_state(this);
    state.writable_types()
}

extern "C" fn writing_options_for_type(
    _this: &Object,
    _sel: Sel,
    _type: id,
    _pastaboard: id,
) -> NSUInteger {
    1 << 9 // NSPasteboardWritingPromised
}

extern "C" fn pasteboard_property_list_for_type(this: &Object, _sel: Sel, ty: id) -> id {
    let state = item_state(this);
    state.object_for_type(ty)
}

extern "C" fn file_promise_file_name_for_type(
    this: &Object,
    _sel: Sel,
    _provider: id,
    file_type: id,
) -> id {
    let state = item_state(this);
    state.file_promise_file_name_for_type(file_type)
}

extern "C" fn file_promise_write_to_url(
    this: &Object,
    _sel: Sel,
    _provider: id,
    url: id,
    completion_block: id,
) {
    let completion_block = unsafe { &mut *(completion_block as *mut Block<(id,), ()>) };
    let completion_block = unsafe { RcBlock::copy(completion_block) };
    let completion_fn = move |error: id| {
        unsafe { completion_block.call((error,)) };
    };
    let state = item_state(this);
    state.file_promise_write_to_url(url, Box::new(completion_fn));
}

extern "C" fn dealloc(this: &Object, _sel: Sel) {
    unsafe {
        let state_ptr = {
            let state_ptr: *mut c_void = *this.get_ivar("sneState");
            state_ptr as *const ItemState
        };
        Rc::from_raw(state_ptr);

        let superclass = superclass(this);
        let () = msg_send![super(this, superclass), dealloc];
    }
}

static PASTEBOARD_WRITER_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("SNEPasteboardWriter", superclass).unwrap();
    decl.add_ivar::<*mut c_void>("sneState");
    if let Some(protocol) = Protocol::get("NSPasteboardWriting") {
        decl.add_protocol(protocol);
    }
    if let Some(protocol) = Protocol::get("NSFilePromiseProviderDelegate") {
        decl.add_protocol(protocol);
    }
    decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));

    // NSPasteboardWriting
    decl.add_method(
        sel!(writableTypesForPasteboard:),
        writable_types_for_pasteboard as extern "C" fn(&Object, Sel, id) -> id,
    );
    decl.add_method(
        sel!(writingOptionsForType:pasteboard:),
        writing_options_for_type as extern "C" fn(&Object, Sel, id, id) -> NSUInteger,
    );
    decl.add_method(
        sel!(pasteboardPropertyListForType:),
        pasteboard_property_list_for_type as extern "C" fn(&Object, Sel, id) -> id,
    );

    // NSFilePromiseProviderDelegate
    decl.add_method(
        sel!(filePromiseProvider:fileNameForType:),
        file_promise_file_name_for_type as extern "C" fn(&Object, Sel, id, id) -> id,
    );
    decl.add_method(
        sel!(filePromiseProvider:writePromiseToURL:completionHandler:),
        file_promise_write_to_url as extern "C" fn(&Object, Sel, id, id, id),
    );

    decl.register()
});
