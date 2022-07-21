use std::{fmt::Display, panic::Location};

use log::{Level, Record};
use nativeshell_core::{MethodCallError, SendMessageError};

use crate::error::NativeExtensionsError;

fn log_error<E: Display>(err: E, location: &Location) {
    log::logger().log(
        &Record::builder()
            .args(format_args!("Unexpected error {} at {}", err, location))
            .file(Some(location.file()))
            .line(Some(location.line()))
            .level(Level::Error)
            .build(),
    );
}

pub trait OkLog<T> {
    fn ok_log(self) -> Option<T>;
}

impl<T, E> OkLog<T> for std::result::Result<T, E>
where
    E: Display,
{
    #[track_caller]
    fn ok_log(self) -> Option<T> {
        match self {
            Ok(value) => Some(value),
            Err(err) => {
                let location = Location::caller();
                log_error(err, location);
                None
            }
        }
    }
}

pub trait OkLogUnexpected<T> {
    fn ok_log_unexpected(self) -> Option<T>;
}

impl<T> OkLogUnexpected<T> for std::result::Result<T, NativeExtensionsError> {
    #[track_caller]
    fn ok_log_unexpected(self) -> Option<T> {
        match self {
            Ok(value) => Some(value),
            Err(err) => {
                if let NativeExtensionsError::MethodCallError(err) = &err {
                    if let MethodCallError::SendError(err) = err {
                        match err {
                            // These are expected errors during isolate shutdown and
                            // do not need to be logged
                            SendMessageError::MessageRefused => return None,
                            SendMessageError::IsolateShutDown => return None,
                            _ => {}
                        }
                    }
                }
                let location = Location::caller();
                log_error(err, location);
                None
            }
        }
    }
}
