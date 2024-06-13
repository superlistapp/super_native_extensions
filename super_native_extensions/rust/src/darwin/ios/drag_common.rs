use objc2_ui_kit::UIDropOperation;

use crate::api_model::DropOperation;

pub trait DropOperationExt {
    fn to_platform(&self) -> UIDropOperation;
    fn from_platform(operation: UIDropOperation) -> DropOperation;
}

impl DropOperationExt for DropOperation {
    fn to_platform(&self) -> UIDropOperation {
        match self {
            DropOperation::None => UIDropOperation::Cancel,
            DropOperation::UserCancelled => UIDropOperation::Cancel,
            DropOperation::Forbidden => UIDropOperation::Forbidden,
            DropOperation::Copy => UIDropOperation::Copy,
            DropOperation::Move => UIDropOperation::Move,
            DropOperation::Link => UIDropOperation::Cancel,
        }
    }

    fn from_platform(operation: UIDropOperation) -> DropOperation {
        #[allow(non_upper_case_globals)]
        match operation {
            UIDropOperation::Cancel => DropOperation::None,
            UIDropOperation::Forbidden => DropOperation::Forbidden,
            UIDropOperation::Copy => DropOperation::Copy,
            UIDropOperation::Move => DropOperation::Move,
            _ => DropOperation::None,
        }
    }
}
