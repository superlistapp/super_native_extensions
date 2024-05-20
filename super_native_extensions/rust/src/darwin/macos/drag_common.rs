use objc2_app_kit::NSDragOperation;

use crate::api_model::DropOperation;

pub trait DropOperationExt {
    fn to_platform(&self) -> NSDragOperation;
    fn from_platform(operation: NSDragOperation) -> DropOperation;
    fn from_platform_mask(operation_mask: NSDragOperation) -> Vec<DropOperation>;
}

impl DropOperationExt for DropOperation {
    fn to_platform(&self) -> NSDragOperation {
        match self {
            DropOperation::None => NSDragOperation::None,
            DropOperation::UserCancelled => NSDragOperation::None,
            DropOperation::Forbidden => NSDragOperation::None,
            DropOperation::Copy => NSDragOperation::Copy,
            DropOperation::Link => NSDragOperation::Link,
            DropOperation::Move => NSDragOperation::Move,
        }
    }

    fn from_platform(operation: NSDragOperation) -> DropOperation {
        #[allow(non_upper_case_globals)]
        match operation {
            NSDragOperation::Copy => DropOperation::Copy,
            NSDragOperation::Move => DropOperation::Move,
            NSDragOperation::Link => DropOperation::Link,
            _ => DropOperation::None,
        }
    }

    fn from_platform_mask(operation_mask: NSDragOperation) -> Vec<DropOperation> {
        let mut res = Vec::new();
        if operation_mask.0 & NSDragOperation::Move.0 == NSDragOperation::Move.0 {
            res.push(DropOperation::Move);
        }
        if operation_mask.0 & NSDragOperation::Copy.0 == NSDragOperation::Copy.0 {
            res.push(DropOperation::Copy);
        }
        if operation_mask.0 & NSDragOperation::Link.0 == NSDragOperation::Link.0 {
            res.push(DropOperation::Link);
        }
        res
    }
}
