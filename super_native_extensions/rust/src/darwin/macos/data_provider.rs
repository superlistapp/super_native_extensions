use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fs::File,
    io::Write,
    mem::ManuallyDrop,
    os::{
        raw::c_void,
        unix::prelude::{FromRawFd, IntoRawFd},
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

use nativeshell_core::{
    platform::{run_loop::PollSession, value::ValueObjcConversion},
    util::Late,
    Context, IsolateId,
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
    api_model::{DataProvider, DataProviderValueId, DataRepresentation},
    data_provider_manager::{
        DataProviderHandle, PlatformDataProviderDelegate, VirtualFileResult, VirtualSessionHandle,
    },
    error::NativeExtensionsResult,
    log::OkLog,
    platform_impl::platform::common::{
        from_nsstring, path_from_url, superclass, to_nserror, to_nsstring,
    },
    value_promise::ValuePromiseResult,
};

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

pub struct PlatformDataProvider {
    weak_self: Late<Weak<Self>>,
    delegate: Weak<dyn PlatformDataProviderDelegate>,
    isolate_id: IsolateId,
    data: DataProvider,
}

thread_local! {
    static WAITING_FOR_PASTEBOARD_DATA: Cell<bool> = Cell::new(false);
}

impl PlatformDataProvider {
    pub fn new(
        delegate: Weak<dyn PlatformDataProviderDelegate>,
        isolate_id: IsolateId,
        data: DataProvider,
    ) -> Self {
        Self {
            delegate,
            data,
            isolate_id,
            weak_self: Late::new(),
        }
    }

    pub fn set_waiting_for_pasteboard_data(waiting: bool) {
        WAITING_FOR_PASTEBOARD_DATA.with(|f| f.set(waiting));
    }

    pub fn is_waiting_for_pasteboard_data() -> bool {
        WAITING_FOR_PASTEBOARD_DATA.with(|f| f.get())
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    /// If retain_handle is false, writer will not retain the DataProviderHandle. This is useful
    /// for drag and drop where the item will live in dragging pasteboard after drag sessions is done.
    pub fn create_writer(
        &self,
        handle: Arc<DataProviderHandle>,
        retain_handle: bool,
        is_for_dragging: bool,
    ) -> StrongPtr {
        let state = Rc::new(ItemState {
            data_provider: self.weak_self.clone(),
            data_provider_handle: Arc::downgrade(&handle),
            _retained_data_provider_handle: if retain_handle {
                Some(handle.clone())
            } else {
                None
            },
            virtual_files: RefCell::new(Vec::new()),
            is_for_dragging,
        });
        state.create_item()
    }

    pub async fn write_to_clipboard(
        providers: Vec<(Rc<PlatformDataProvider>, Arc<DataProviderHandle>)>,
    ) -> NativeExtensionsResult<()> {
        autoreleasepool(|| unsafe {
            let items: Vec<_> = providers
                .into_iter()
                .map(|p| p.0.create_writer(p.1, true, false).autorelease())
                .collect();
            let array = NSArray::arrayWithObjects(nil, &items);
            let pasteboard = NSPasteboard::generalPasteboard(nil);
            NSPasteboard::clearContents(pasteboard);
            NSPasteboard::writeObjects(pasteboard, array);
        });
        Ok(())
    }
}

struct ItemState {
    data_provider: Weak<PlatformDataProvider>,
    data_provider_handle: std::sync::Weak<DataProviderHandle>,
    _retained_data_provider_handle: Option<Arc<DataProviderHandle>>,
    virtual_files: RefCell<Vec<Arc<VirtualSessionHandle>>>,
    is_for_dragging: bool,
}

struct VirtualFileInfo {
    id: DataProviderValueId,
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
        Class::get("NSFilePromiseProvider")?;
        self.data_provider.upgrade().and_then(|data_provider| {
            let data = &data_provider.data;
            data.representations.iter().find_map(|item| match item {
                DataRepresentation::VirtualFile {
                    id,
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
        match self.data_provider.upgrade() {
            Some(data_provider) => {
                let data = &data_provider.data;
                let types: Vec<_> = data
                    .representations
                    .iter()
                    .filter_map(|d| match d {
                        DataRepresentation::Simple { format, data: _ } => {
                            Some(to_nsstring(format).autorelease())
                        }
                        DataRepresentation::Lazy { format, id: _ } => {
                            Some(to_nsstring(format).autorelease())
                        }
                        _ => None,
                    })
                    .collect();
                // Dragging will fail with empty pasteboard. But it is a valid
                // use case in case we have only local data
                if types.is_empty() && self.is_for_dragging {
                    unsafe {
                        NSArray::arrayWithObject(
                            nil,
                            *to_nsstring("dev.nativeshell.placeholder-item"),
                        )
                    }
                } else {
                    unsafe { NSArray::arrayWithObjects(nil, &types) }
                }
            }
            None => nil,
        }
    }

    fn object_for_type(&self, pasteboard_type: id) -> id {
        match self.data_provider.upgrade() {
            Some(data_provider) => {
                let ty = unsafe { from_nsstring(pasteboard_type) };
                let data = &data_provider.data;
                for repr in &data.representations {
                    match repr {
                        DataRepresentation::Simple { format, data } => {
                            if &ty == format {
                                return data
                                    .to_objc()
                                    .ok_log()
                                    .map(|a| a.autorelease())
                                    .unwrap_or(nil);
                            }
                        }
                        DataRepresentation::Lazy { format, id } => {
                            if &ty == format {
                                if let Some(delegate) = data_provider.delegate.upgrade() {
                                    let promise =
                                        delegate.get_lazy_data(data_provider.isolate_id, *id, None);
                                    let mut poll_session = PollSession::new();
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
                                        PlatformDataProvider::set_waiting_for_pasteboard_data(true);
                                        Context::get()
                                            .run_loop()
                                            .platform_run_loop
                                            .poll_once(&mut poll_session);
                                        PlatformDataProvider::set_waiting_for_pasteboard_data(
                                            false,
                                        );
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
        match self.data_provider.upgrade() {
            Some(data_provider) => {
                let data = &data_provider.data;
                data.suggested_name
                    .as_ref()
                    .map(|name| to_nsstring(name).autorelease())
                    .unwrap_or(nil)
            }
            None => nil,
        }
    }

    fn progress_for_url(url: id) -> StrongPtr {
        unsafe {
            let progress = StrongPtr::retain(
                msg_send![class!(NSProgress), discreteProgressWithTotalUnitCount: 1000u64],
            );
            let () = msg_send![*progress, setKind:*to_nsstring("NSProgressKindFile")];
            let () = msg_send![*progress, setFileURL: url];
            let () = msg_send![*progress, setCancellable: YES];
            let () = msg_send![*progress, publish];
            progress
        }
    }

    fn file_promise_do_write(
        self: &Rc<Self>,
        url: id,
        completion_fn: Box<dyn FnOnce(id)>,
        info: VirtualFileInfo,
        data_provider: Rc<PlatformDataProvider>,
        delegate: Rc<dyn PlatformDataProviderDelegate>,
        data_provider_handle: Arc<DataProviderHandle>,
    ) {
        let progress = Self::progress_for_url(url);

        let path = path_from_url(url);
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
        FILE_PATHS.lock().unwrap().insert(descriptor, path);

        let progress_clone = progress.clone();
        let progress_clone2 = progress.clone();
        let notifier = delegate.get_virtual_file(
            data_provider.isolate_id,
            info.id,
            descriptor,
            Box::new(|_| {}),
            Box::new(move |cnt| {
                let completed = (cnt * 1000.0).round() as u64;
                let () = unsafe { msg_send![*progress_clone, setCompletedUnitCount: completed] };
            }),
            Box::new(move |result| {
                let _handle = data_provider_handle;
                unsafe {
                    let () = msg_send![*progress_clone2, unpublish];
                }
                match result {
                    VirtualFileResult::Done => completion_fn(nil),
                    VirtualFileResult::Error { message } => {
                        let error = to_nserror("super_dnd", 0, &message);
                        completion_fn(*error);
                    }
                    VirtualFileResult::Cancelled => {
                        let error = to_nserror("super_dnd", 0, "Cancelled");
                        completion_fn(*error);
                    }
                }
            }),
        );
        self.virtual_files.borrow_mut().push(notifier.clone());
        let notifier = Arc::downgrade(&notifier);
        let cancellation_handler = ConcreteBlock::new(move || {
            if let Some(notifier) = notifier.upgrade() {
                notifier.dispose();
            }
        });
        let cancellation_handler = cancellation_handler.copy();
        unsafe {
            let () = msg_send![*progress, setCancellationHandler:&*cancellation_handler];
        }
    }

    fn file_promise_write_to_url(self: &Rc<Self>, url: id, completion_fn: Box<dyn FnOnce(id)>) {
        let info = self.virtual_file_info();
        let data_provider = self.data_provider.upgrade();
        let delegate = data_provider.as_ref().and_then(|c| c.delegate.upgrade());
        let data_provider_handle = self.data_provider_handle.upgrade();

        match (info, data_provider, delegate, data_provider_handle) {
            (Some(info), Some(clipboard), Some(delegate), Some(drop_notifier)) => {
                self.file_promise_do_write(
                    url,
                    completion_fn,
                    info,
                    clipboard,
                    delegate,
                    drop_notifier,
                );
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
