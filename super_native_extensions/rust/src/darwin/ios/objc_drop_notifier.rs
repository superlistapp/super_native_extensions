use std::{os::raw::c_void, sync::Arc};

use cocoa::base::id;
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    rc::StrongPtr,
    runtime::{Class, Object, Sel},
    sel, sel_impl,
};
use once_cell::sync::Lazy;

use crate::util::DropNotifier;

use super::{superclass, util::IntoObjc};

extern "C" fn dealloc(this: &Object, _sel: Sel) {
    unsafe {
        let state_ptr = {
            let state_ptr: *const c_void = *this.get_ivar("state");
            state_ptr as *const DropNotifier
        };
        Arc::from_raw(state_ptr);

        let superclass = superclass(this);
        let () = msg_send![super(this, superclass), dealloc];
    }
}

static DROP_NOTIFIER_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("SNEDropNotifier", superclass).unwrap();

    decl.add_ivar::<*mut c_void>("state");
    decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));

    decl.register()
});

impl IntoObjc for Arc<DropNotifier> {
    fn into_objc(self) -> StrongPtr {
        unsafe {
            let notifier: id = msg_send![*DROP_NOTIFIER_CLASS, new];
            (*notifier).set_ivar("state", Arc::into_raw(self) as *const c_void);
            StrongPtr::new(notifier)
        }
    }
}
