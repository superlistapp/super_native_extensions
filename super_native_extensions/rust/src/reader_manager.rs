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
    platform::PlatformDataReader,
    util::NextId,
};

pub struct DataReaderManager {
    weak_self: Late<Weak<Self>>,
    invoker: Late<AsyncMethodInvoker>,
    next_id: Cell<i64>,
    readers: RefCell<HashMap<i64, ReaderEntry>>,
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

impl DataReaderManager {
    pub fn new() -> RegisteredAsyncMethodHandler<Self> {
        Self {
            weak_self: Late::new(),
            invoker: Late::new(),
            next_id: Cell::new(1),
            readers: RefCell::new(HashMap::new()),
        }
        .register("DataReaderManager")
    }

    pub fn register_platform_reader(
        &self,
        platform_reader: Rc<PlatformDataReader>,
        isolate_id: IsolateId,
    ) -> RegisteredDataReader {
        let id = self.next_id.next_id();
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

    fn dispose_reader(&self, reader: i64) -> NativeExtensionsResult<()> {
        self.readers.borrow_mut().remove(&reader);
        Ok(())
    }

    async fn get_items(&self, reader: i64) -> NativeExtensionsResult<Vec<i64>> {
        let reader = self
            .readers
            .borrow()
            .get(&reader)
            .map(|r| r.platform_reader.clone());
        match reader {
            Some(reader) => reader.get_items().await,
            None => Err(NativeExtensionsError::ReaderNotFound),
        }
    }

    async fn get_item_formats(
        &self,
        request: ItemFormatsRequest,
    ) -> NativeExtensionsResult<Vec<String>> {
        let reader = self
            .readers
            .borrow()
            .get(&request.reader_handle)
            .map(|r| r.platform_reader.clone());
        match reader {
            Some(reader) => reader.get_formats_for_item(request.item_handle).await,
            None => Err(NativeExtensionsError::ReaderNotFound),
        }
    }

    async fn get_item_data(&self, request: ItemDataRequest) -> NativeExtensionsResult<Value> {
        let reader = self
            .readers
            .borrow()
            .get(&request.reader_handle)
            .map(|r| r.platform_reader.clone());
        match reader {
            Some(reader) => {
                reader
                    .get_data_for_item(request.item_handle, request.format)
                    .await
            }
            None => Err(NativeExtensionsError::ReaderNotFound),
        }
    }
}

#[derive(IntoValue, TryFromValue, Debug, Clone)]
#[nativeshell(rename_all = "camelCase")]
pub struct RegisteredDataReader {
    handle: i64,
    finalizable_handle: Value,
}

#[derive(TryFromValue)]
#[nativeshell(rename_all = "camelCase")]
struct ItemFormatsRequest {
    item_handle: i64,
    reader_handle: i64,
}

#[derive(TryFromValue)]
#[nativeshell(rename_all = "camelCase")]
struct ItemDataRequest {
    item_handle: i64,
    reader_handle: i64,
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
                .get_item_data(call.args.try_into()?)
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
