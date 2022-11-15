use std::rc::Rc;

use async_trait::async_trait;
use irondash_message_channel::{
    AsyncMethodHandler, MethodCall, PlatformError, PlatformResult, RegisteredAsyncMethodHandler,
    Value,
};

use crate::{
    context::Context, platform_impl::platform::PlatformDataReader,
    reader_manager::GetDataReaderManager,
};

pub struct ClipboardReader {}

impl ClipboardReader {
    pub fn new() -> RegisteredAsyncMethodHandler<Self> {
        Self {}.register("ClipboardReader")
    }
}

pub trait GetClipboardReader {
    fn clipboard_reader(&self) -> Rc<ClipboardReader>;
}

impl GetClipboardReader for Context {
    fn clipboard_reader(&self) -> Rc<ClipboardReader> {
        self.get_attachment(ClipboardReader::new).handler()
    }
}

#[async_trait(?Send)]
impl AsyncMethodHandler for ClipboardReader {
    async fn on_method_call(&self, call: MethodCall) -> PlatformResult {
        match call.method.as_str() {
            "newClipboardReader" => {
                let reader = PlatformDataReader::new_clipboard_reader()?;
                Ok(Context::get()
                    .data_reader_manager()
                    .register_platform_reader(reader, call.isolate)
                    .into())
            }
            _ => Err(PlatformError {
                code: "invalid_method".into(),
                message: Some(format!("Unknown Method: {}", call.method)),
                detail: Value::Null,
            }),
        }
    }
}
