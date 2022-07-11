use cocoa::foundation::NSUInteger;

use crate::api_model::DropOperation;

pub type NSDragOperation = NSUInteger;

#[allow(non_upper_case_globals)]
pub const NSDragOperationNone: NSDragOperation = 0;
#[allow(non_upper_case_globals)]
pub const NSDragOperationCopy: NSDragOperation = 1;
#[allow(non_upper_case_globals)]
pub const NSDragOperationLink: NSDragOperation = 2;
#[allow(non_upper_case_globals)]
pub const NSDragOperationMove: NSDragOperation = 16;

pub trait DropOperationExt {
    fn to_platform(&self) -> NSDragOperation;
    fn from_platform(operation: NSDragOperation) -> DropOperation;
    fn from_platform_mask(operation_mask: NSDragOperation) -> Vec<DropOperation>;
}

impl DropOperationExt for DropOperation {
    fn to_platform(&self) -> NSDragOperation {
        match self {
            DropOperation::None => NSDragOperationNone,
            DropOperation::Forbidden => NSDragOperationNone,
            DropOperation::Copy => NSDragOperationCopy,
            DropOperation::Link => NSDragOperationLink,
            DropOperation::Move => NSDragOperationMove,
        }
    }

    fn from_platform(operation: NSDragOperation) -> DropOperation {
        #[allow(non_upper_case_globals)]
        match operation {
            NSDragOperationCopy => DropOperation::Copy,
            NSDragOperationMove => DropOperation::Move,
            NSDragOperationLink => DropOperation::Link,
            _ => DropOperation::None,
        }
    }

    fn from_platform_mask(operation_mask: NSDragOperation) -> Vec<DropOperation> {
        let mut res = Vec::new();
        if operation_mask & NSDragOperationMove == NSDragOperationMove {
            res.push(DropOperation::Move);
        }
        if operation_mask & NSDragOperationCopy == NSDragOperationCopy {
            res.push(DropOperation::Copy);
        }
        if operation_mask & NSDragOperationLink == NSDragOperationLink {
            res.push(DropOperation::Link);
        }
        res
    }
}
