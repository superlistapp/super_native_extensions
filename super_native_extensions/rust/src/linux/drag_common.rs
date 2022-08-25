use gdk::DragAction;

use crate::api_model::DropOperation;

pub trait DropOperationExt {
    fn to_platform(&self) -> DragAction;
    fn from_platform(action: DragAction) -> Self;
    fn from_platform_mask(actions: DragAction) -> Vec<DropOperation>;
}

impl DropOperationExt for DropOperation {
    fn to_platform(&self) -> DragAction {
        match self {
            DropOperation::None => DragAction::empty(),
            DropOperation::UserCancelled => DragAction::empty(),
            DropOperation::Forbidden => DragAction::empty(),
            DropOperation::Copy => DragAction::COPY,
            DropOperation::Move => DragAction::MOVE,
            DropOperation::Link => DragAction::LINK,
        }
    }

    fn from_platform(action: DragAction) -> Self {
        match action {
            DragAction::DEFAULT => Self::Copy,
            DragAction::COPY => Self::Copy,
            DragAction::MOVE => Self::Move,
            DragAction::LINK => Self::Link,
            _ => Self::None,
        }
    }

    fn from_platform_mask(actions: DragAction) -> Vec<DropOperation> {
        let mut res = Vec::new();
        if actions.contains(DragAction::MOVE) {
            res.push(DropOperation::Move);
        }
        if actions.contains(DragAction::COPY) {
            res.push(DropOperation::Copy);
        }
        if actions.contains(DragAction::LINK) {
            res.push(DropOperation::Link);
        }
        res
    }
}
