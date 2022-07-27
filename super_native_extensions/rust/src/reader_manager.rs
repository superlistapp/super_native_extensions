use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    sync::Arc,
};

use async_trait::async_trait;

use nativeshell_core::{
    util::Late, AsyncMethodHandler, AsyncMethodInvoker, Context, FinalizableHandle,
    IntoPlatformResult, IntoValue, IsolateId, MethodCall, PlatformError, PlatformResult,
    RegisteredAsyncMethodHandler, TryFromValue, Value,
};

use crate::{
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
    progresses: RefCell<HashMap<(IsolateId, i64), Weak<ReadProgress>>>,
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

pub struct ReadProgress {
    _drop_notifier: Arc<DropNotifier>,
    cancellation_handler: RefCell<Option<Box<dyn FnOnce()>>>,
    on_set_cancellation_handler: Box<dyn Fn(bool)>,
    on_progress: Box<dyn Fn(Option<f64>)>,
}

impl ReadProgress {
    #[allow(dead_code)]
    pub fn set_cancellation_handler(&self, handler: Option<Box<dyn FnOnce()>>) {
        (self.on_set_cancellation_handler)(handler.is_some());
        self.cancellation_handler.replace(handler);
    }
    #[allow(dead_code)]
    pub fn report_progress(&self, progress: Option<f64>) {
        (self.on_progress)(progress);
    }
    fn cancel(&self) {
        let handler = self.cancellation_handler.replace(None);
        if let Some(handler) = handler {
            handler();
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
        }
        .register("DataReaderManager")
    }

    fn new_read_progress(&self, isolate_id: IsolateId, progress_id: i64) -> Rc<ReadProgress> {
        #[derive(IntoValue)]
        #[nativeshell(rename_all = "camelCase")]
        struct SetProgressCancellable {
            progress_id: i64,
            cancellable: bool,
        }
        #[derive(IntoValue)]
        #[nativeshell(rename_all = "camelCase")]
        struct ProgressUpdate {
            progress_id: i64,
            fraction: Option<f64>,
        }
        let weak_self_1 = self.weak_self.clone();
        let weak_self_2 = self.weak_self.clone();
        let weak_self_3 = self.weak_self.clone();
        let res = Rc::new(ReadProgress {
            _drop_notifier: Arc::new(DropNotifier::new(move || {
                if let Some(this) = weak_self_1.upgrade() {
                    this.progresses
                        .borrow_mut()
                        .remove(&(isolate_id, progress_id));
                }
            })),
            cancellation_handler: RefCell::new(None),
            on_set_cancellation_handler: Box::new(move |cancellable| {
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
            }),
            on_progress: Box::new(move |fraction| {
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
            }),
        });
        self.progresses
            .borrow_mut()
            .insert((isolate_id, progress_id), Rc::downgrade(&res));
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

    async fn get_item_data(
        &self,
        isolate_id: IsolateId,
        request: ItemDataRequest,
    ) -> NativeExtensionsResult<Value> {
        let reader = self.get_reader(request.reader_handle)?;
        let progress = self.new_read_progress(isolate_id, request.progress_id);
        reader
            .get_data_for_item(request.item_handle, request.format, progress)
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

    async fn can_get_virtual_file(
        &self,
        request: VirtualFileSupportedRequest,
    ) -> NativeExtensionsResult<bool> {
        self.get_reader(request.reader_handle)?
            .can_get_virtual_file_for_item(request.item_handle, &request.format)
            .await
    }

    async fn get_virtual_file(
        &self,
        isolate_id: IsolateId,
        request: VirtualFileRequest,
    ) -> NativeExtensionsResult<String> {
        let reader = self.get_reader(request.reader_handle)?;
        let progress = self.new_read_progress(isolate_id, request.progress_id);
        let res = reader
            .get_virtual_file_for_item(
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
#[nativeshell(rename_all = "camelCase")]
pub struct RegisteredDataReader {
    handle: DataReaderId,
    finalizable_handle: Value,
}

#[derive(TryFromValue)]
#[nativeshell(rename_all = "camelCase")]
struct ItemFormatsRequest {
    item_handle: i64,
    reader_handle: DataReaderId,
}

#[derive(TryFromValue)]
#[nativeshell(rename_all = "camelCase")]
struct ItemDataRequest {
    item_handle: i64,
    reader_handle: DataReaderId,
    format: String,
    progress_id: i64,
}

#[derive(TryFromValue)]
#[nativeshell(rename_all = "camelCase")]
struct VirtualFileRequest {
    item_handle: i64,
    reader_handle: DataReaderId,
    format: String,
    progress_id: i64,
    target_folder: String,
}

#[derive(TryFromValue)]
#[nativeshell(rename_all = "camelCase")]
struct VirtualFileSupportedRequest {
    item_handle: i64,
    reader_handle: DataReaderId,
    format: String,
}

#[async_trait(?Send)]
impl AsyncMethodHandler for DataReaderManager {
    fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    fn assign_invoker(&self, invoker: AsyncMethodInvoker) {
        self.invoker.set(invoker);
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
            "getItemData" => self
                .get_item_data(call.isolate, call.args.try_into()?)
                .await
                .into_platform_result(),
            "cancelProgress" => self
                .cancel_progress(call.isolate, call.args.try_into()?)
                .into_platform_result(),
            "canGetVirtualFile" => self
                .can_get_virtual_file(call.args.try_into()?)
                .await
                .into_platform_result(),
            "getVirtualFile" => self
                .get_virtual_file(call.isolate, call.args.try_into()?)
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
