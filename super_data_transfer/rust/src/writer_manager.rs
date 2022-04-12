use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    sync::Arc,
};

use async_trait::async_trait;
use nativeshell_core::{
    AsyncMethodHandler, AsyncMethodInvoker, Context, IntoPlatformResult, IsolateId, MethodCall,
    PlatformError, PlatformResult, Value, util::Late,
};

use crate::{
    error::{ClipboardError, ClipboardResult},
    platform_impl::PlatformClipboardWriter,
    value_promise::{ValuePromise, ValuePromiseResult},
    writer_data::ClipboardWriterData,
};

pub struct ClipboardWriterManager {
    weak_self: Late<Weak<Self>>,
    invoker: Late<AsyncMethodInvoker>,
    next_id: Cell<i64>,
    writers: RefCell<HashMap<i64, WriterEntry>>,
}

struct WriterEntry {
    isolate_id: IsolateId,
    platform_writer: Rc<PlatformClipboardWriter>,
}

impl ClipboardWriterManager {
    pub fn new() -> Self {
        Self {
            weak_self: Late::new(),
            invoker: Late::new(),
            next_id: Cell::new(1),
            writers: RefCell::new(HashMap::new()),
        }
    }

    fn register_writer(
        &self,
        data: ClipboardWriterData,
        isolate_id: IsolateId,
    ) -> ClipboardResult<i64> {
        let id = self.next_id.get();
        self.next_id.replace(id + 1);
        let platform_clipboard = Rc::new(PlatformClipboardWriter::new(
            self.weak_self.clone(),
            isolate_id,
            data,
        ));
        platform_clipboard.assign_weak_self(Rc::downgrade(&platform_clipboard));
        self.writers.borrow_mut().insert(
            id,
            WriterEntry {
                isolate_id,
                platform_writer: platform_clipboard,
            },
        );
        Ok(id)
    }

    fn unregister_writer(&self, writer: i64) -> ClipboardResult<()> {
        self.writers.borrow_mut().remove(&writer);
        Ok(())
    }

    async fn write_to_clipboard(&self, clipboard: i64) -> ClipboardResult<()> {
        let clipboard = self
            .writers
            .borrow()
            .get(&clipboard)
            .map(|e| e.platform_writer.clone());
        if let Some(clipboard) = clipboard {
            clipboard.write_to_clipboard().await
        } else {
            Err(ClipboardError::OtherError("Clipboard not found".into()))
        }
    }
}

#[async_trait(?Send)]
impl AsyncMethodHandler for ClipboardWriterManager {
    async fn on_method_call(&self, call: MethodCall) -> PlatformResult {
        match call.method.as_str() {
            "registerClipboardWriter" => self
                .register_writer(call.args.try_into()?, call.isolate)
                .into_platform_result(),
            "unregisterClipboardWriter" => self
                .unregister_writer(call.args.try_into()?)
                .into_platform_result(),
            "writeToClipboard" => self
                .write_to_clipboard(call.args.try_into()?)
                .await
                .into_platform_result(),
            _ => Err(PlatformError {
                code: "invalid_method".into(),
                message: None,
                detail: Value::Null,
            }),
        }
    }

    fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    fn assign_invoker(&self, invoker: AsyncMethodInvoker) {
        self.invoker.set(invoker);
    }

    // Called when engine is about to be destroyed.
    fn on_isolate_destroyed(&self, isolate_id: IsolateId) {
        let mut writers = self.writers.borrow_mut();
        let writers_to_remove: Vec<_> = writers
            .iter()
            .filter_map(|(id, writer)| {
                if writer.isolate_id == isolate_id {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        for writer_id in writers_to_remove {
            writers.remove(&writer_id);
        }
    }
}

#[async_trait(?Send)]
pub trait PlatformClipboardWriterDelegate {
    fn get_lazy_data(
        &self,
        isolate_id: IsolateId,
        data_id: i64,
        on_done: Option<Box<dyn FnOnce()>>,
    ) -> Arc<ValuePromise>;
    async fn get_lazy_data_async(&self, isolate_id: IsolateId, data_id: i64) -> ValuePromiseResult;
}

#[async_trait(?Send)]
impl PlatformClipboardWriterDelegate for ClipboardWriterManager {
    fn get_lazy_data(
        &self,
        isolate_id: IsolateId,
        data_id: i64,
        on_done: Option<Box<dyn FnOnce()>>,
    ) -> Arc<ValuePromise> {
        let res = Arc::new(ValuePromise::new());
        let res_clone = res.clone();
        let invoker = self.invoker.clone();
        Context::get().run_loop().spawn(async move {
            let res = invoker
                .call_method_cv(isolate_id, "getLazyData", data_id)
                .await;
            if let Some(on_done) = on_done {
                on_done();
            }
            match res {
                Ok(res) => res_clone.set(res),
                Err(_) => res_clone.cancel(),
            }
        });
        res
    }
    async fn get_lazy_data_async(&self, isolate_id: IsolateId, data_id: i64) -> ValuePromiseResult {
        let res = self
            .invoker
            .call_method_cv(isolate_id, "getLazyData", data_id)
            .await;
        match res {
            Ok(res) => res,
            Err(_) => ValuePromiseResult::Cancelled,
        }
    }
}
