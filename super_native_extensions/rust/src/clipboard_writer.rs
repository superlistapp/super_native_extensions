use std::{
    rc::{Rc, Weak},
    sync::Arc,
};

use async_trait::async_trait;
use irondash_message_channel::{
    AsyncMethodHandler, AsyncMethodInvoker, IntoPlatformResult, IsolateId, Late, MethodCall,
    PlatformError, PlatformResult, RegisteredAsyncMethodHandler, Value,
};

use crate::{
    api_model::DataProviderId, context::Context, data_provider_manager::GetDataProviderManager,
    error::NativeExtensionsResult, log::OkLog, platform_impl::platform::PlatformDataProvider,
    util::DropNotifier,
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

    fn release_data_provider(&self, isolate_id: IsolateId, provider_id: DataProviderId) {
        self.invoker
            .call_method_sync(isolate_id, "releaseDataProvider", provider_id, |r| {
                r.ok_log();
            })
    }

    async fn write_to_clipboard(
        &self,
        isolate_id: IsolateId,
        provider_ids: Vec<DataProviderId>,
    ) -> NativeExtensionsResult<()> {
        let mut providers = Vec::<_>::new();
        let data_provider_manager = Context::get().data_provider_manager();
        for provider_id in provider_ids {
            let provider = data_provider_manager.get_platform_data_provider(provider_id)?;
            let weak_self = self.weak_self.clone();
            let notifier = DropNotifier::new(move || {
                if let Some(this) = weak_self.upgrade() {
                    this.release_data_provider(isolate_id, provider_id);
                }
            });
            providers.push((provider, Arc::new(notifier.into())));
        }
        PlatformDataProvider::write_to_clipboard(providers).await?;
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
