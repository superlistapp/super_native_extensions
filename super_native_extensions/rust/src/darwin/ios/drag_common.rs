use crate::api_model::DropOperation;

use super::uikit::{
    UIDropOperation, UIDropOperationCancel, UIDropOperationCopy, UIDropOperationForbidden,
    UIDropOperationMove,
};

pub trait DropOperationExt {
    fn to_platform(&self) -> UIDropOperation;
    fn from_platform(operation: UIDropOperation) -> DropOperation;
}

impl DropOperationExt for DropOperation {
    fn to_platform(&self) -> UIDropOperation {
        match self {
            DropOperation::None => UIDropOperationCancel,
            DropOperation::UserCancelled => UIDropOperationCancel,
            DropOperation::Forbidden => UIDropOperationForbidden,
            DropOperation::Copy => UIDropOperationCopy,
            DropOperation::Move => UIDropOperationMove,
            DropOperation::Link => UIDropOperationCancel,
        }
    }

    fn from_platform(operation: UIDropOperation) -> DropOperation {
        #[allow(non_upper_case_globals)]
        match operation {
            UIDropOperationCancel => DropOperation::None,
            UIDropOperationForbidden => DropOperation::Forbidden,
            UIDropOperationCopy => DropOperation::Copy,
            UIDropOperationMove => DropOperation::Move,
            _ => DropOperation::None,
        }
    }
}
