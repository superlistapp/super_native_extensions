use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    sync::{self, Arc, Mutex},
};

use async_trait::async_trait;

use irondash_message_channel::{
    AsyncMethodHandler, AsyncMethodInvoker, FinalizableHandle, IntoPlatformResult, IntoValue,
    IsolateId, Late, MethodCall, PlatformError, PlatformResult, RegisteredAsyncMethodHandler,
    TryFromValue, Value,
};
use irondash_run_loop::{util::Capsule, RunLoop, RunLoopSender};

use crate::{
    context::Context,
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    platform::PlatformDataReader,
    util::{DropNotifier, NextId},
};

#[derive(Debug, TryFromValue, IntoValue, Clone, Copy, PartialEq, Hash, Eq)]
struct DataReaderId(i64);

impl From<i64> for DataReaderId {
    fn from(i: i64) -> Self {
        Self(i)
    }
}

pub struct DataReaderManager {
    weak_self: Late<Weak<Self>>,
    invoker: Late<AsyncMethodInvoker>,
    next_id: Cell<i64>,
    readers: RefCell<HashMap<DataReaderId, ReaderEntry>>,
    progresses: RefCell<HashMap<(IsolateId, i64), sync::Weak<ReadProgress>>>,
    virtual_file_readers: RefCell<HashMap<(IsolateId, i64), Rc<dyn VirtualFileReader>>>,
}

struct ReaderEntry {
    platform_reader: Rc<PlatformDataReader>,
    _finalizable_handle: Arc<FinalizableHandle>,
}

pub trait GetDataReaderManager {
    fn data_reader_manager(&self) -> Rc<DataReaderManager>;
}

impl GetDataReaderManager for Context {
    fn data_reader_manager(&self) -> Rc<DataReaderManager> {
        self.get_attachment(DataReaderManager::new).handler()
    }
}

struct ReadProgressInner {
    cancellation_handler: Option<Box<dyn FnOnce() + Send>>,
    on_set_cancellation_handler: Box<dyn Fn(bool /* is cancellable */)>,
    on_progress: Box<dyn Fn(Option<f64>)>,
}

pub struct ReadProgress {
    _drop_notifier: Arc<DropNotifier>,
    sender: RunLoopSender,
    inner: Mutex<Capsule<ReadProgressInner>>,
}

/// Progress is thread safe. It must be created on main thread. Callbacks
/// specified in constructor are guaranteed to be invoked on main thread.
impl ReadProgress {
    fn new<F1, F2>(
        drop_notifier: Arc<DropNotifier>,
        on_set_cancellation_handler: F1,
        on_progress: F2,
    ) -> Self
    where
        F1: Fn(bool) + 'static,
        F2: Fn(Option<f64>) + 'static,
    {
        Self {
            _drop_notifier: drop_notifier,
            sender: RunLoop::current().new_sender(),
            inner: Mutex::new(Capsule::new_with_sender(
                ReadProgressInner {
                    cancellation_handler: None,
                    on_set_cancellation_handler: Box::new(on_set_cancellation_handler),
                    on_progress: Box::new(on_progress),
                },
                RunLoop::current().new_sender(),
            )),
        }
    }

    #[allow(dead_code)]
    pub fn set_cancellation_handler(self: &Arc<Self>, handler: Option<Box<dyn FnOnce() + Send>>) {
        if Context::current().is_some() {
            let mut inner = self.inner.lock().unwrap();
            let inner = inner.get_mut().unwrap();
            (inner.on_set_cancellation_handler)(handler.is_some());
            inner.cancellation_handler = handler;
        } else {
            let self_clone = self.clone();
            self.sender.send(move || {
                self_clone.set_cancellation_handler(handler);
            });
        }
    }
    #[allow(dead_code)]
    pub fn report_progress(self: &Arc<Self>, fraction: Option<f64>) {
        if Context::current().is_some() {
            let inner = self.inner.lock().unwrap();
            let inner = inner.get_ref().unwrap();
            (inner.on_progress)(fraction);
        } else {
            let self_clone = self.clone();
            self.sender.send(move || {
                self_clone.report_progress(fraction);
            });
        }
    }

    fn cancel(self: &Arc<Self>) {
        if Context::current().is_some() {
            let mut inner = self.inner.lock().unwrap();
            let inner = inner.get_mut().unwrap();
            let handler = inner.cancellation_handler.take();
            if let Some(handler) = handler {
                handler();
            }
        } else {
            let self_clone = self.clone();
            self.sender.send(move || {
                self_clone.cancel();
            });
        }
    }
}

impl DataReaderManager {
    pub fn new() -> RegisteredAsyncMethodHandler<Self> {
        Self {
            weak_self: Late::new(),
            invoker: Late::new(),
            next_id: Cell::new(1),
            readers: RefCell::new(HashMap::new()),
            progresses: RefCell::new(HashMap::new()),
            virtual_file_readers: RefCell::new(HashMap::new()),
        }
        .register("DataReaderManager")
    }

    fn new_read_progress(&self, isolate_id: IsolateId, progress_id: i64) -> Arc<ReadProgress> {
        #[derive(IntoValue)]
        #[irondash(rename_all = "camelCase")]
        struct SetProgressCancellable {
            progress_id: i64,
            cancellable: bool,
        }
        #[derive(IntoValue)]
        #[irondash(rename_all = "camelCase")]
        struct ProgressUpdate {
            progress_id: i64,
            fraction: Option<f64>,
        }
        let weak_self_1 = self.weak_self.clone();
        let weak_self_2 = self.weak_self.clone();
        let weak_self_3 = self.weak_self.clone();
        let res = Arc::new(ReadProgress::new(
            Arc::new(DropNotifier::new(move || {
                if let Some(this) = weak_self_1.upgrade() {
                    this.progresses
                        .borrow_mut()
                        .remove(&(isolate_id, progress_id));
                }
            })),
            move |cancellable| {
                if let Some(this) = weak_self_2.upgrade() {
                    this.invoker.call_method_sync(
                        isolate_id,
                        "setProgressCancellable",
                        SetProgressCancellable {
                            progress_id,
                            cancellable,
                        },
                        |r| {
                            r.ok_log();
                        },
                    );
                }
            },
            move |fraction| {
                if let Some(this) = weak_self_3.upgrade() {
                    this.invoker.call_method_sync(
                        isolate_id,
                        "updateProgress",
                        ProgressUpdate {
                            progress_id,
                            fraction,
                        },
                        |r| {
                            r.ok_log();
                        },
                    );
                }
            },
        ));
        self.progresses
            .borrow_mut()
            .insert((isolate_id, progress_id), Arc::downgrade(&res));
        res
    }

    pub fn register_platform_reader(
        &self,
        platform_reader: Rc<PlatformDataReader>,
        isolate_id: IsolateId,
    ) -> RegisteredDataReader {
        let id: DataReaderId = self.next_id.next_id().into();
        let weak_self = self.weak_self.clone();
        let finalizable_handle = Arc::new(FinalizableHandle::new(32, isolate_id, move || {
            if let Some(manager) = weak_self.upgrade() {
                manager.readers.borrow_mut().remove(&id);
            }
        }));

        self.readers.borrow_mut().insert(
            id,
            ReaderEntry {
                platform_reader,
                _finalizable_handle: finalizable_handle.clone(),
            },
        );

        RegisteredDataReader {
            handle: id,
            finalizable_handle: finalizable_handle.into(),
        }
    }

    fn dispose_reader(&self, reader: DataReaderId) -> NativeExtensionsResult<()> {
        self.readers.borrow_mut().remove(&reader);
        Ok(())
    }

    fn get_reader(&self, reader: DataReaderId) -> NativeExtensionsResult<Rc<PlatformDataReader>> {
        if let Some(entry) = self.readers.borrow().get(&reader) {
            Ok(entry.platform_reader.clone())
        } else {
            Err(NativeExtensionsError::ReaderNotFound)
        }
    }

    async fn get_items(&self, reader: DataReaderId) -> NativeExtensionsResult<Vec<i64>> {
        self.get_reader(reader)?.get_items().await
    }

    async fn get_item_formats(
        &self,
        request: ItemFormatsRequest,
    ) -> NativeExtensionsResult<Vec<String>> {
        self.get_reader(request.reader_handle)?
            .get_formats_for_item(request.item_handle)
            .await
    }

    async fn item_format_is_synthetized(
        &self,
        request: ItemFormatIsSynthetizedRequest,
    ) -> NativeExtensionsResult<bool> {
        self.get_reader(request.reader_handle)?
            .item_format_is_synthetized(request.item_handle, &request.format)
    }

    async fn get_item_suggested_name(
        &self,
        request: ItemSuggestedNameRequest,
    ) -> NativeExtensionsResult<Option<String>> {
        self.get_reader(request.reader_handle)?
            .get_suggested_name_for_item(request.item_handle)
            .await
    }

    async fn get_format_for_file_uri(&self, uri: String) -> NativeExtensionsResult<Option<String>> {
        PlatformDataReader::get_format_for_file_uri(uri).await
    }

    async fn get_item_data(
        &self,
        isolate_id: IsolateId,
        request: ItemDataRequest,
    ) -> NativeExtensionsResult<Value> {
        let reader = self.get_reader(request.reader_handle)?;
        let progress = self.new_read_progress(isolate_id, request.progress_id);
        reader
            .get_data_for_item(request.item_handle, request.format, Some(progress))
            .await
    }

    fn cancel_progress(
        &self,
        isolate_id: IsolateId,
        progress_id: i64,
    ) -> NativeExtensionsResult<()> {
        let progress = self
            .progresses
            .borrow_mut()
            .remove(&(isolate_id, progress_id));
        if let Some(progress) = progress.and_then(|p| p.upgrade()) {
            progress.cancel();
        }
        Ok(())
    }

    async fn can_copy_virtual_file(
        &self,
        request: VirtualFileSupportedRequest,
    ) -> NativeExtensionsResult<bool> {
        self.get_reader(request.reader_handle)?
            .can_copy_virtual_file_for_item(request.item_handle, &request.format)
            .await
    }

    async fn can_read_virtual_file(
        &self,
        request: VirtualFileSupportedRequest,
    ) -> NativeExtensionsResult<bool> {
        self.get_reader(request.reader_handle)?
            .can_read_virtual_file_for_item(request.item_handle, &request.format)
            .await
    }

    async fn virtual_file_reader_create(
        &self,
        isolate_id: IsolateId,
        request: VirtualFileReaderRequest,
    ) -> NativeExtensionsResult<VirtualFileReaderResponse> {
        let reader = self.get_reader(request.reader_handle)?;
        let progress = self.new_read_progress(isolate_id, request.progress_id);
        let res = reader
            .create_virtual_file_reader_for_item(request.item_handle, &request.format, progress)
            .await?;
        match res {
            Some(reader) => {
                let reader_handle = self.next_id.next_id();
                let file_size = reader.file_size()?;
                let file_name = reader.file_name();
                self.virtual_file_readers
                    .borrow_mut()
                    .insert((isolate_id, reader_handle), reader);
                Ok(VirtualFileReaderResponse {
                    reader_handle,
                    file_name,
                    file_size,
                })
            }
            None => Err(NativeExtensionsError::VirtualFileReceiveError(
                "not supported".into(),
            )),
        }
    }

    async fn virtual_file_reader_read(
        &self,
        isolate_id: IsolateId,
        virtual_reader_id: i64,
    ) -> NativeExtensionsResult<Option<Vec<u8>>> {
        let reader = self
            .virtual_file_readers
            .borrow()
            .get(&(isolate_id, virtual_reader_id))
            .cloned();
        match reader {
            Some(reader) => reader.read_next().await.map(Some),
            None => Ok(None),
        }
    }

    fn virtual_file_reader_close(
        &self,
        isolate_id: IsolateId,
        virtual_reader_id: i64,
    ) -> NativeExtensionsResult<()> {
        let reader = self
            .virtual_file_readers
            .borrow_mut()
            .remove(&(isolate_id, virtual_reader_id));
        if let Some(reader) = reader {
            reader.close()?;
        }
        Ok(())
    }

    async fn copy_virtual_file(
        &self,
        isolate_id: IsolateId,
        request: VirtualFileCopyRequest,
    ) -> NativeExtensionsResult<String> {
        let reader = self.get_reader(request.reader_handle)?;
        let progress = self.new_read_progress(isolate_id, request.progress_id);
        let res = reader
            .copy_virtual_file_for_item(
                request.item_handle,
                &request.format,
                request.target_folder.into(),
                progress,
            )
            .await?;
        Ok(res.to_string_lossy().into_owned())
    }
}

#[derive(IntoValue, TryFromValue, Debug, Clone)]
#[irondash(rename_all = "camelCase")]
pub struct RegisteredDataReader {
    handle: DataReaderId,
    finalizable_handle: Value,
}

#[derive(TryFromValue)]
#[irondash(rename_all = "camelCase")]
struct ItemFormatsRequest {
    item_handle: i64,
    reader_handle: DataReaderId,
}

#[derive(TryFromValue)]
#[irondash(rename_all = "camelCase")]
struct ItemFormatIsSynthetizedRequest {
    item_handle: i64,
    reader_handle: DataReaderId,
    format: String,
}

#[derive(TryFromValue)]
#[irondash(rename_all = "camelCase")]
struct ItemSuggestedNameRequest {
    item_handle: i64,
    reader_handle: DataReaderId,
}

#[derive(TryFromValue)]
#[irondash(rename_all = "camelCase")]
struct ItemDataRequest {
    item_handle: i64,
    reader_handle: DataReaderId,
    format: String,
    progress_id: i64,
}

#[derive(TryFromValue)]
#[irondash(rename_all = "camelCase")]
struct VirtualFileReaderRequest {
    item_handle: i64,
    reader_handle: DataReaderId,
    format: String,
    progress_id: i64,
}

#[derive(IntoValue)]
#[irondash(rename_all = "camelCase")]
struct VirtualFileReaderResponse {
    reader_handle: i64,
    file_size: Option<i64>,
    file_name: Option<String>,
}

#[derive(TryFromValue)]
#[irondash(rename_all = "camelCase")]
struct VirtualFileCopyRequest {
    item_handle: i64,
    reader_handle: DataReaderId,
    format: String,
    progress_id: i64,
    target_folder: String,
}

#[derive(TryFromValue)]
#[irondash(rename_all = "camelCase")]
struct VirtualFileSupportedRequest {
    item_handle: i64,
    reader_handle: DataReaderId,
    format: String,
}

#[async_trait(?Send)]
pub trait VirtualFileReader {
    async fn read_next(&self) -> NativeExtensionsResult<Vec<u8>>;
    fn file_size(&self) -> NativeExtensionsResult<Option<i64>>;
    fn file_name(&self) -> Option<String>;
    fn close(&self) -> NativeExtensionsResult<()>;
}

#[async_trait(?Send)]
impl AsyncMethodHandler for DataReaderManager {
    fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    fn assign_invoker(&self, invoker: AsyncMethodInvoker) {
        self.invoker.set(invoker);
    }

    fn on_isolate_destroyed(&self, destroyed_isolate_id: IsolateId) {
        let mut progresses = self.progresses.borrow_mut();
        progresses.retain(|(isolate_id, _), progress| {
            if *isolate_id == destroyed_isolate_id {
                if let Some(progress) = progress.upgrade() {
                    progress.cancel();
                }
                false
            } else {
                true
            }
        });

        let mut readers = self.virtual_file_readers.borrow_mut();
        readers.retain(|(isolate_id, _), reader| {
            if *isolate_id == destroyed_isolate_id {
                reader.close().ok_log();
                false
            } else {
                true
            }
        })
    }

    async fn on_method_call(&self, call: MethodCall) -> PlatformResult {
        match call.method.as_str() {
            "disposeReader" => self
                .dispose_reader(call.args.try_into()?)
                .into_platform_result(),
            "getItems" => self
                .get_items(call.args.try_into()?)
                .await
                .into_platform_result(),
            "getItemFormats" => self
                .get_item_formats(call.args.try_into()?)
                .await
                .into_platform_result(),
            "itemFormatIsSynthetized" => self
                .item_format_is_synthetized(call.args.try_into()?)
                .await
                .into_platform_result(),
            "getItemSuggestedName" => self
                .get_item_suggested_name(call.args.try_into()?)
                .await
                .into_platform_result(),
            "getItemData" => self
                .get_item_data(call.isolate, call.args.try_into()?)
                .await
                .into_platform_result(),
            "getFormatForFileUri" => self
                .get_format_for_file_uri(call.args.try_into()?)
                .await
                .into_platform_result(),
            "cancelProgress" => self
                .cancel_progress(call.isolate, call.args.try_into()?)
                .into_platform_result(),
            "canCopyVirtualFile" => self
                .can_copy_virtual_file(call.args.try_into()?)
                .await
                .into_platform_result(),
            "canReadVirtualFile" => self
                .can_read_virtual_file(call.args.try_into()?)
                .await
                .into_platform_result(),
            "virtualFileReaderCreate" => self
                .virtual_file_reader_create(call.isolate, call.args.try_into()?)
                .await
                .into_platform_result(),
            "virtualFileReaderRead" => self
                .virtual_file_reader_read(call.isolate, call.args.try_into()?)
                .await
                .into_platform_result(),
            "virtualFileReaderClose" => self
                .virtual_file_reader_close(call.isolate, call.args.try_into()?)
                .into_platform_result(),
            "copyVirtualFile" => self
                .copy_virtual_file(call.isolate, call.args.try_into()?)
                .await
                .into_platform_result(),
            _ => Err(PlatformError {
                code: "invalid_method".into(),
                message: Some(format!("Unknown Method: {}", call.method)),
                detail: Value::Null,
            }),
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::ClipboardReaderManager;
//     use crate::{platform::READERS, reader_manager::NewClipboardReaderResult};
//     use nativeshell_core::{Context, FinalizableHandle, GetMessageChannel, MockIsolate, Value};
//     use std::{sync::Arc, time::Duration};

//     async fn test_dispose_main() {
//         let _reader_manager = ClipboardReaderManager::new();
//         let context = Context::get();
//         let channel = "ClipboardReaderManager";

//         let isolate_1 = MockIsolate::new();
//         let isolate_1 = isolate_1.attach(&context.message_channel());

//         assert_eq!(READERS.with(|c| c.borrow().len()), 0);

//         //
//         // Finalizable handle
//         //

//         let reader_id: NewClipboardReaderResult = isolate_1
//             .call_method_async(channel, "newDefaultReader", Value::Null)
//             .await
//             .unwrap()
//             .try_into()
//             .unwrap();

//         assert_eq!(READERS.with(|c| c.borrow().len()), 1);

//         let handle: Arc<FinalizableHandle> = reader_id.finalizable_handle.try_into().unwrap();
//         // Simulate finalizing handle
//         handle.finalize();

//         // wait one run loop turn
//         context.run_loop().wait(Duration::from_secs(0)).await;

//         assert_eq!(READERS.with(|c| c.borrow().len()), 0);

//         //
//         // disposeReader call
//         //

//         let reader_id: NewClipboardReaderResult = isolate_1
//             .call_method_async(channel, "newDefaultReader", Value::Null)
//             .await
//             .unwrap()
//             .try_into()
//             .unwrap();

//         assert_eq!(READERS.with(|c| c.borrow().len()), 1);

//         isolate_1
//             .call_method_async(channel, "disposeReader", reader_id.handle.into())
//             .await
//             .unwrap();

//         assert_eq!(READERS.with(|c| c.borrow().len()), 0);

//         //
//         // Removing isolate
//         //

//         isolate_1
//             .call_method_async(channel, "newDefaultReader", Value::Null)
//             .await
//             .unwrap();

//         assert_eq!(READERS.with(|c| c.borrow().len()), 1);

//         drop(isolate_1);

//         context.run_loop().wait(Duration::from_secs(0)).await;

//         assert_eq!(READERS.with(|c| c.borrow().len()), 0);
//     }

//     #[test]
//     fn test_dispose() {
//         Context::run_test(test_dispose_main());
//     }
// }
