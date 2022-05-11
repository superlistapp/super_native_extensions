use std::fmt::Display;

use nativeshell_core::{PlatformError, Value};

#[derive(Debug)]
pub enum ClipboardError {
    UnknownError,
    OtherError(String),
    ReaderNotFound,
}

pub type ClipboardResult<T> = Result<T, ClipboardError>;

impl Display for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClipboardError::UnknownError => write!(f, "unknown error"),
            ClipboardError::OtherError(m) => write!(f, "{:?}", m),
            ClipboardError::ReaderNotFound => write!(f, "platform reader not found"),
        }
    }
}

impl std::error::Error for ClipboardError {}

impl From<ClipboardError> for PlatformError {
    fn from(err: ClipboardError) -> Self {
        PlatformError {
            code: "clipboard_error".into(),
            message: Some(err.to_string()),
            detail: Value::Null,
        }
    }
}
