use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fs::File,
    io::Write,
    mem::ManuallyDrop,
    os::unix::prelude::{FromRawFd, IntoRawFd},
    path::PathBuf,
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
};

use block2::{Block, RcBlock};
use irondash_message_channel::{value_darwin::ValueObjcConversion, IsolateId, Late};
use irondash_run_loop::{platform::PollSession, RunLoop};
use objc2::{
    declare_class, extern_class, extern_methods, msg_send_id,
    mutability::{self, InteriorMutable},
    rc::{Allocated, Id},
    runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject},
    ClassType, DeclaredClass,
};
use objc2_app_kit::{
    NSFilePromiseProvider, NSFilePromiseProviderDelegate, NSPasteboard, NSPasteboardType,
    NSPasteboardWriting, NSPasteboardWritingOptions,
};
use objc2_foundation::{NSArray, NSError, NSProgress, NSProgressKindFile, NSString, NSURL};
use once_cell::sync::Lazy;

use crate::{
    api_model::{DataProvider, DataProviderValueId, DataRepresentation},
    data_provider_manager::{
        DataProviderHandle, PlatformDataProviderDelegate, VirtualFileResult, VirtualSessionHandle,
    },
    error::NativeExtensionsResult,
    log::OkLog,
    platform_impl::platform::common::{path_from_url, to_nserror},
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
            std::fs::remove_file(path).ok();
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
    static WAITING_FOR_PASTEBOARD_DATA: Cell<bool> = const { Cell::new(false) };
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
    ) -> Id<NSObject> {
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
        let items: Vec<_> = providers
            .into_iter()
            .map(|p| p.0.create_writer(p.1, true, false))
            .collect();
        let array = NSArray::from_vec(items);
        let pasteboard = unsafe { NSPasteboard::generalPasteboard() };
        unsafe { pasteboard.clearContents() };
        unsafe { pasteboard.writeObjects(&Id::cast(array)) };
        Ok(())
    }
}

pub struct ItemState {
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
    fn create_item(self: Rc<Self>) -> Id<NSObject> {
        let writer = SNEPasteboardWriter::alloc();
        let writer = writer.set_ivars(Ivars {
            item_state: self.clone(),
        });
        let writer: Id<SNEPasteboardWriter> = unsafe { msg_send_id![super(writer), init] };

        let info = self.virtual_file_info();

        match info {
            Some(info) => unsafe {
                let provider = SNEForwardingFilePromiseProvider::init(
                    SNEForwardingFilePromiseProvider::alloc(),
                );
                provider.setFileType(&NSString::from_str(&info.format));
                provider.setDelegate(Some(&Id::cast(writer.clone())));
                provider.setWritingDelegate(Some(&Id::cast(writer)));
                Id::cast(provider)
            },
            None => unsafe { Id::cast(writer) },
        }
    }

    fn virtual_file_info(self: &Rc<Self>) -> Option<VirtualFileInfo> {
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

    fn writable_types(&self) -> Id<NSArray<NSPasteboardType>> {
        match self.data_provider.upgrade() {
            Some(data_provider) => {
                let data = &data_provider.data;
                let types: Vec<_> = data
                    .representations
                    .iter()
                    .filter_map(|d| match d {
                        DataRepresentation::Simple { format, data: _ } => {
                            Some(NSString::from_str(format))
                        }
                        DataRepresentation::Lazy { format, id: _ } => {
                            Some(NSString::from_str(format))
                        }
                        _ => None,
                    })
                    .collect();
                // Dragging will fail with empty pasteboard. But it is a valid
                // use case in case we have only local data
                if types.is_empty() && self.is_for_dragging {
                    NSArray::from_vec(vec![NSString::from_str("dev.nativeshell.placeholder-item")])
                } else {
                    NSArray::from_vec(types)
                }
            }
            None => unsafe { NSArray::array() },
        }
    }

    fn object_for_type(&self, pasteboard_type: &NSPasteboardType) -> Option<Id<NSObject>> {
        match self.data_provider.upgrade() {
            Some(data_provider) => {
                let ty = pasteboard_type.to_string();
                let data = &data_provider.data;
                for repr in &data.representations {
                    match repr {
                        DataRepresentation::Simple { format, data } => {
                            if &ty == format {
                                return data.to_objc().ok_log().flatten();
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
                                                    return value.to_objc().ok_log().flatten()
                                                }
                                                ValuePromiseResult::Cancelled => {
                                                    return None;
                                                }
                                            }
                                        }
                                        PlatformDataProvider::set_waiting_for_pasteboard_data(true);
                                        RunLoop::current()
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
                None
            }
            None => None,
        }
    }

    fn file_promise_file_name_for_type(self: &Rc<Self>, _file_type: &NSString) -> Id<NSString> {
        match self.data_provider.upgrade() {
            Some(data_provider) => {
                let data = &data_provider.data;
                data.suggested_name
                    .as_ref()
                    .map(|name| NSString::from_str(name))
                    .unwrap_or(unsafe { NSString::string() })
            }
            None => unsafe { NSString::string() },
        }
    }

    fn progress_for_url(url: &NSURL) -> Id<NSProgress> {
        unsafe {
            let progress = NSProgress::initWithParent_userInfo(NSProgress::alloc(), None, None);
            progress.setKind(Some(NSProgressKindFile));
            progress.setFileURL(Some(url));
            progress.setCancellable(true);
            progress.publish();
            progress
        }
    }

    fn file_promise_do_write(
        self: &Rc<Self>,
        url: &NSURL,
        completion_fn: Box<dyn FnOnce(Option<Id<NSError>>)>,
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
                completion_fn(Some(error));
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
                let completed = (cnt * 1000.0).round() as i64;
                unsafe { progress_clone.setCompletedUnitCount(completed) };
            }),
            Box::new(move |result| {
                let _handle = data_provider_handle;
                unsafe { progress_clone2.unpublish() };
                match result {
                    VirtualFileResult::Done => completion_fn(None),
                    VirtualFileResult::Error { message } => {
                        let error = to_nserror("super_dnd", 0, &message);
                        completion_fn(Some(error));
                    }
                    VirtualFileResult::Cancelled => {
                        let error = to_nserror("super_dnd", 0, "Cancelled");
                        completion_fn(Some(error));
                    }
                }
            }),
        );
        self.virtual_files.borrow_mut().push(notifier.clone());
        let notifier = Arc::downgrade(&notifier);
        let cancellation_handler = RcBlock::new(move || {
            if let Some(notifier) = notifier.upgrade() {
                notifier.dispose();
            }
        });
        unsafe {
            progress.setCancellationHandler(Some(&cancellation_handler));
        }
    }

    fn file_promise_write_to_url(
        self: &Rc<Self>,
        url: &NSURL,
        completion_fn: Box<dyn FnOnce(Option<Id<NSError>>)>,
    ) {
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
                completion_fn(Some(error));
            }
        }
    }
}

struct Ivars {
    item_state: Rc<ItemState>,
}

declare_class!(
    struct SNEPasteboardWriter;

    unsafe impl ClassType for SNEPasteboardWriter {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
        const NAME: &'static str = "SNEPasteboardWriter";
    }

    impl DeclaredClass for SNEPasteboardWriter {
        type Ivars = Ivars;
    }

    unsafe impl NSObjectProtocol for SNEPasteboardWriter {}

    unsafe impl NSPasteboardWriting for SNEPasteboardWriter {
        #[method_id(writableTypesForPasteboard:)]
        #[allow(non_snake_case)]
        unsafe fn writableTypesForPasteboard(
            &self,
            _pasteboard: &NSPasteboard,
        ) -> Id<NSArray<NSPasteboardType>> {
            self.ivars().item_state.writable_types()
        }

        #[method(writingOptionsForType:pasteboard:)]
        #[allow(non_snake_case)]
        unsafe fn writingOptionsForType_pasteboard(
            &self,
            r#_type: &NSPasteboardType,
            _pasteboard: &NSPasteboard,
        ) -> NSPasteboardWritingOptions {
            NSPasteboardWritingOptions::NSPasteboardWritingPromised
        }

        #[method_id(pasteboardPropertyListForType:)]
        #[allow(non_snake_case)]
        unsafe fn pasteboardPropertyListForType(
            &self,
            r#type: &NSPasteboardType,
        ) -> Option<Id<AnyObject>> {
            self.ivars().item_state.object_for_type(r#type).map(|v| Id::cast(v))
        }
    }

    unsafe impl NSFilePromiseProviderDelegate for SNEPasteboardWriter {
        #[method_id(filePromiseProvider:fileNameForType:)]
        #[allow(non_snake_case)]
        unsafe fn filePromiseProvider_fileNameForType(
            &self,
            _file_promise_provider: &NSFilePromiseProvider,
            file_type: &NSString,
        ) -> Id<NSString> {
            self.ivars().item_state.file_promise_file_name_for_type(file_type)
        }

        #[method(filePromiseProvider:writePromiseToURL:completionHandler:)]
        #[allow(non_snake_case)]
        unsafe fn filePromiseProvider_writePromiseToURL_completionHandler(
            &self,
            _file_promise_provider: &NSFilePromiseProvider,
            url: &NSURL,
            completion_handler: &Block<dyn Fn(*mut NSError)>,
        ) {
            let completion_handler =
                RcBlock::<dyn Fn(*mut NSError)>::copy(completion_handler as *const _ as *mut _).unwrap();
            let completion_fn = move |error: Option<Id<NSError>>| {
                let error = match error {
                    Some(error) => Id::as_ptr(&error),
                    None => std::ptr::null_mut(),
                };
                completion_handler.call((error as *mut _,));
            };
            self.ivars().item_state
                .file_promise_write_to_url(url, Box::new(completion_fn));
        }
    }

    unsafe impl SNEPasteboardWriter {}
);

extern_class!(
    #[derive(PartialEq, Eq, Hash)]
    pub struct SNEForwardingFilePromiseProvider;

    unsafe impl ClassType for SNEForwardingFilePromiseProvider {
        type Super = NSFilePromiseProvider;
        type Mutability = InteriorMutable;
    }
);

extern_methods!(
    unsafe impl SNEForwardingFilePromiseProvider {
        #[allow(non_snake_case)]
        #[method(setWritingDelegate:)]
        pub unsafe fn setWritingDelegate(
            &self,
            delgate: Option<&ProtocolObject<dyn NSPasteboardWriting>>,
        );

        #[method_id(@__retain_semantics Init init)]
        pub unsafe fn init(this: Allocated<Self>) -> Id<Self>;
    }
);
