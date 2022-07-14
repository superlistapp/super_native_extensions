use windows::Win32::System::Ole::{DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE};

use crate::api_model::DropOperation;

pub trait DropOperationExt {
    fn to_platform(&self) -> u32;
    fn from_platform(operation: u32) -> DropOperation;
}

impl DropOperationExt for DropOperation {
    fn to_platform(&self) -> u32 {
        match self {
            DropOperation::None => 0,
            DropOperation::UserCancelled => 0,
            DropOperation::Forbidden => 0,
            DropOperation::Copy => DROPEFFECT_COPY,
            DropOperation::Move => DROPEFFECT_MOVE,
            DropOperation::Link => DROPEFFECT_LINK,
        }
    }

    fn from_platform(operation: u32) -> DropOperation {
        match operation {
            DROPEFFECT_COPY => DropOperation::Copy,
            DROPEFFECT_LINK => DropOperation::Link,
            DROPEFFECT_MOVE => DropOperation::Move,
            _ => DropOperation::None,
        }
    }
}
