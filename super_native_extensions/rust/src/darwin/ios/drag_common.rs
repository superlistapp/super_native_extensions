use cocoa::foundation::NSUInteger;

use crate::api_model::DropOperation;

pub type UIDropOperation = NSUInteger;

#[allow(non_upper_case_globals)]
pub const UIDropOperationCancel: UIDropOperation = 0;
#[allow(non_upper_case_globals)]
pub const UIDropOperationForbidden: UIDropOperation = 1;
#[allow(non_upper_case_globals)]
pub const UIDropOperationCopy: UIDropOperation = 2;
#[allow(non_upper_case_globals)]
pub const UIDropOperationMove: UIDropOperation = 3;

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
