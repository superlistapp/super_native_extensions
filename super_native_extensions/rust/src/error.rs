use std::{fmt::Display, io};

use irondash_message_channel::{MethodCallError, PlatformError, Value};

#[derive(Debug)]
pub enum NativeExtensionsError {
    UnknownError,
    MethodCallError(MethodCallError),
    OtherError(String),
    DataSourceNotFound,
    ReaderNotFound,
    PlatformContextNotFound,
    UnsupportedOperation,
    VirtualFileSessionNotFound,
    VirtualFileReceiveError(String),
    IOError(io::Error),
    InvalidData,
    DragSessionNotFound,
    MouseEventNotFound,
    EngineContextError(irondash_engine_context::Error),
    PlatformMenuNotFound,
    InvalidMenuElement,
    InvalidMenuConfigurationId,
}

pub type NativeExtensionsResult<T> = Result<T, NativeExtensionsError>;

impl Display for NativeExtensionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NativeExtensionsError::UnknownError => write!(f, "unknown error"),
            NativeExtensionsError::MethodCallError(e) => e.fmt(f),
            NativeExtensionsError::OtherError(m) => write!(f, "{m:?}"),
            NativeExtensionsError::DataSourceNotFound => {
                write!(f, "Platform data source not found")
            }
            NativeExtensionsError::ReaderNotFound => write!(f, "platform reader not found"),
            NativeExtensionsError::PlatformContextNotFound => {
                write!(f, "platform drag and drop context not found")
            }
            NativeExtensionsError::UnsupportedOperation => write!(f, "unsupported operation"),
            NativeExtensionsError::VirtualFileSessionNotFound => {
                write!(f, "virtual session not found")
            }
            NativeExtensionsError::VirtualFileReceiveError(m) => {
                write!(f, "virtual file receive error: {m}")
            }
            NativeExtensionsError::IOError(e) => e.fmt(f),
            NativeExtensionsError::InvalidData => write!(f, "invalid data"),
            NativeExtensionsError::DragSessionNotFound => write!(f, "drag session not found"),
            NativeExtensionsError::MouseEventNotFound => write!(f, "mouse event not found"),
            NativeExtensionsError::EngineContextError(e) => e.fmt(f),
            NativeExtensionsError::PlatformMenuNotFound => write!(f, "platform menu not found"),
            NativeExtensionsError::InvalidMenuElement => write!(f, "invalid menu element"),
            NativeExtensionsError::InvalidMenuConfigurationId => {
                write!(f, "invalid menu configuration id")
            }
        }
    }
}

impl std::error::Error for NativeExtensionsError {}

impl NativeExtensionsError {
    fn get_detail(&self) -> Value {
        match self {
            NativeExtensionsError::UnknownError => "unknownError".into(),
            NativeExtensionsError::MethodCallError(_) => "methodCallError".into(),
            NativeExtensionsError::OtherError(_) => "otherError".into(),
            NativeExtensionsError::DataSourceNotFound => "dataSourceNotFound".into(),
            NativeExtensionsError::ReaderNotFound => "readerNotFound".into(),
            NativeExtensionsError::PlatformContextNotFound => "platformContextNotFound".into(),
            NativeExtensionsError::UnsupportedOperation => "unsupportedOperation".into(),
            NativeExtensionsError::VirtualFileSessionNotFound => {
                "virtualFileSessionNotFound".into()
            }
            NativeExtensionsError::VirtualFileReceiveError(_) => "virtualFileReceiveError".into(),
            NativeExtensionsError::IOError(_) => "ioError".into(),
            NativeExtensionsError::InvalidData => "invalidData".into(),
            NativeExtensionsError::DragSessionNotFound => "dragSessionNotFound".into(),
            NativeExtensionsError::MouseEventNotFound => "mouseEventNotFound".into(),
            NativeExtensionsError::EngineContextError(_) => "engineContextError".into(),
            NativeExtensionsError::PlatformMenuNotFound => "platformMenuNotFound".into(),
            NativeExtensionsError::InvalidMenuElement => "invalidMenuElement".into(),
            NativeExtensionsError::InvalidMenuConfigurationId => {
                "invalidMenuConfigurationId".into()
            }
        }
    }
}

impl From<NativeExtensionsError> for PlatformError {
    fn from(err: NativeExtensionsError) -> Self {
        PlatformError {
            code: "super_native_extensions_error".into(),
            message: Some(err.to_string()),
            detail: err.get_detail(),
        }
    }
}

impl From<MethodCallError> for NativeExtensionsError {
    fn from(error: MethodCallError) -> Self {
        NativeExtensionsError::MethodCallError(error)
    }
}

impl From<io::Error> for NativeExtensionsError {
    fn from(e: io::Error) -> Self {
        NativeExtensionsError::IOError(e)
    }
}

impl From<irondash_engine_context::Error> for NativeExtensionsError {
    fn from(e: irondash_engine_context::Error) -> Self {
        NativeExtensionsError::EngineContextError(e)
    }
}
