use std::fmt::Display;

use nativeshell_core::{PlatformError, Value};

#[derive(Debug)]
pub enum NativeExtensionsError {
    UnknownError,
    OtherError(String),
    DataSourceNotFound,
    ReaderNotFound,
    PlatformContextNotFound,
    UnsupportedOperation,
    VirtualFileSessionNotFound,
}

pub type NativeExtensionsResult<T> = Result<T, NativeExtensionsError>;

impl Display for NativeExtensionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NativeExtensionsError::UnknownError => write!(f, "unknown error"),
            NativeExtensionsError::OtherError(m) => write!(f, "{:?}", m),
            NativeExtensionsError::DataSourceNotFound => {
                write!(f, "Platform data source not found")
            }
            NativeExtensionsError::ReaderNotFound => write!(f, "platform reader not found"),
            NativeExtensionsError::PlatformContextNotFound => {
                write!(f, "platform drag drop context not found")
            }
            NativeExtensionsError::UnsupportedOperation => write!(f, "unsupported operation"),
            NativeExtensionsError::VirtualFileSessionNotFound => {
                write!(f, "virtual session not found")
            }
        }
    }
}

impl std::error::Error for NativeExtensionsError {}

impl From<NativeExtensionsError> for PlatformError {
    fn from(err: NativeExtensionsError) -> Self {
        PlatformError {
            code: "super_native_extensions_error".into(),
            message: Some(err.to_string()),
            detail: Value::Null,
        }
    }
}
