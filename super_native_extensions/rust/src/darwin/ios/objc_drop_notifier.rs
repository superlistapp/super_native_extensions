use std::sync::Arc;

use objc2::{
    declare::{Ivar, IvarDrop},
    declare_class, msg_send_id, mutability,
    rc::Id,
    runtime::NSObject,
    ClassType,
};

use crate::util::DropNotifier;

use super::util::IntoObjc;

impl IntoObjc for Arc<DropNotifier> {
    fn into_objc(self) -> Id<NSObject> {
        Id::into_super(SNEDropNotifier::new(self))
    }
}

declare_class!(
    struct SNEDropNotifier {
        drop_notifier: IvarDrop<Box<Arc<DropNotifier>>, "_drop_notifier">,
    }

    mod ivars;

    unsafe impl ClassType for SNEDropNotifier {
        type Super = NSObject;
        type Mutability = mutability::Mutable;
        const NAME: &'static str = "SNEDropNotifier";
    }
);

impl SNEDropNotifier {
    fn new(drop_notifier: Arc<DropNotifier>) -> Id<Self> {
        let mut this: Id<Self> = unsafe { msg_send_id![Self::alloc(), init] };
        Ivar::write(&mut this.drop_notifier, Box::new(drop_notifier));
        this
    }
}
