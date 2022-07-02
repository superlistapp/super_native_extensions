use std::{
    rc::{Rc, Weak},
    sync::Arc,
};

use async_trait::async_trait;
use nativeshell_core::{
    util::Late, AsyncMethodHandler, AsyncMethodInvoker, Context, IntoPlatformResult, IsolateId,
    MethodCall, PlatformError, PlatformResult, RegisteredAsyncMethodHandler, Value,
};

use crate::{
    api_model::DataSourceId, data_source_manager::GetDataSourceManager,
    error::NativeExtensionsResult, util::DropNotifier,
};

pub struct ClipboardWriter {
    weak_self: Late<Weak<Self>>,
    invoker: Late<AsyncMethodInvoker>,
}

impl ClipboardWriter {
    pub fn new() -> RegisteredAsyncMethodHandler<Self> {
        Self {
            weak_self: Late::new(),
            invoker: Late::new(),
        }
        .register("ClipboardWriter")
    }

    fn on_dropped(&self, isolate_id: IsolateId, source_id: DataSourceId) {
        self.invoker
            .call_method_sync(isolate_id, "releaseDataSource", source_id, |_| {})
    }

    async fn write_to_clipboard(
        &self,
        isolate_id: IsolateId,
        source_id: DataSourceId,
    ) -> NativeExtensionsResult<()> {
        let source = Context::get()
            .data_source_manager()
            .get_platform_data_source(source_id)?;
        let weak_self = self.weak_self.clone();
        source
            .write_to_clipboard(DropNotifier::new(move || {
                if let Some(this) = weak_self.upgrade() {
                    this.on_dropped(isolate_id, source_id);
                }
            }))
            .await?;
        Ok(())
    }
}

pub trait GetClipboardWriter {
    fn clipboard_writer(&self) -> Rc<ClipboardWriter>;
}

impl GetClipboardWriter for Context {
    fn clipboard_writer(&self) -> Rc<ClipboardWriter> {
        self.get_attachment(ClipboardWriter::new).handler()
    }
}

#[async_trait(?Send)]
impl AsyncMethodHandler for ClipboardWriter {
    async fn on_method_call(&self, call: MethodCall) -> PlatformResult {
        match call.method.as_str() {
            "writeToClipboard" => self
                .write_to_clipboard(call.isolate, call.args.try_into()?)
                .await
                .into_platform_result(),
            _ => Err(PlatformError {
                code: "invalid_method".into(),
                message: Some(format!("Unknown Method: {}", call.method)),
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
}
