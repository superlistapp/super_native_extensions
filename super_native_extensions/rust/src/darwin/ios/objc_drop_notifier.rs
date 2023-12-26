use std::sync::Arc;

use objc2::{
    declare_class, msg_send_id, mutability, rc::Id, runtime::NSObject, ClassType, DeclaredClass,
};

use crate::util::DropNotifier;

use super::util::IntoObjc;

impl IntoObjc for Arc<DropNotifier> {
    fn into_objc(self) -> Id<NSObject> {
        Id::into_super(SNEDropNotifier::new(self))
    }
}

struct Ivars {
    _notifier: Arc<DropNotifier>,
}

declare_class!(
    struct SNEDropNotifier;

    unsafe impl ClassType for SNEDropNotifier {
        type Super = NSObject;
        type Mutability = mutability::Mutable;
        const NAME: &'static str = "SNEDropNotifier";
    }

    impl DeclaredClass for SNEDropNotifier {
        type Ivars = Ivars;
    }
);

impl SNEDropNotifier {
    fn new(drop_notifier: Arc<DropNotifier>) -> Id<Self> {
        let this = Self::alloc();
        let this = this.set_ivars(Ivars {
            _notifier: drop_notifier,
        });
        unsafe { msg_send_id![super(this), init] }
    }
}
