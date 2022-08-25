use std::{ffi::c_void, mem::ManuallyDrop, rc::Rc, sync::Arc};

use cocoa::{
    base::{id, nil, BOOL, YES},
    foundation::NSUInteger,
};

use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    rc::{StrongPtr, WeakPtr},
    runtime::{Class, Object, Sel},
    sel, sel_impl,
};
use once_cell::sync::Lazy;

use crate::{
    platform_impl::platform::common::{
        objc_setAssociatedObject, to_nsstring, OBJC_ASSOCIATION_RETAIN,
    },
    reader_manager::ReadProgress,
    util::Movable,
};

use super::common::superclass;

static POLICY_KEY: char = 'k';

/// Bridges NSPRogress to ReadProgress. Will retain ReadProgress for as long as the
/// NSProgress is alive.
#[allow(dead_code)]
pub unsafe fn bridge_progress(ns_progress: id, read_progress: Arc<ReadProgress>) {
    let bridge: id = msg_send![*PROGRESS_BRIDGE_CLASS, new];
    let bridge = StrongPtr::new(bridge);
    let state = Rc::new(State {
        ns_progress,
        read_progress: read_progress.clone(),
    });
    (**bridge).set_ivar("state", Rc::into_raw(state) as *const c_void);
    #[allow(non_upper_case_globals)]
    const NSKeyValueObservingOptionInitial: NSUInteger = 0x04;
    let () = msg_send![ns_progress,
                        addObserver: *bridge
                        forKeyPath: *to_nsstring("fractionCompleted")
                        options: NSKeyValueObservingOptionInitial
                        context: nil];
    let key = &POLICY_KEY as *const _ as *const c_void;
    objc_setAssociatedObject(ns_progress, key, *bridge, OBJC_ASSOCIATION_RETAIN);

    let cancellable: BOOL = msg_send![ns_progress, isCancellable];
    if cancellable == YES {
        let weak = Movable::new(WeakPtr::new(ns_progress));
        read_progress.set_cancellation_handler(Some(Box::new(move || {
            let progress = weak.load();
            if *progress != nil {
                let () = msg_send![*progress, cancel];
            }
        })));
    } else {
        read_progress.set_cancellation_handler(None)
    }
}

struct State {
    ns_progress: id,
    read_progress: Arc<ReadProgress>,
}

extern "C" fn dealloc(this: &Object, _sel: Sel) {
    unsafe {
        let state_ptr = {
            let state_ptr: *const c_void = *this.get_ivar("state");
            state_ptr as *const State
        };
        let state = Rc::from_raw(state_ptr);
        let () = msg_send![state.ns_progress,
                removeObserver: this
                forKeyPath:*to_nsstring("fractionCompleted")
                context:nil];
        let superclass = superclass(this);
        let () = msg_send![super(this, superclass), dealloc];
    }
}

extern "C" fn observe_value_for_key_path(
    this: &mut Object,
    _sel: Sel,
    _key_path: id,
    object: id,
    _change: id,
    _context: *mut c_void,
) {
    let state_ptr = unsafe {
        let state_ptr: *const c_void = *this.get_ivar("state");
        state_ptr as *const State
    };
    let state = ManuallyDrop::new(unsafe { Rc::from_raw(state_ptr) });
    let completed: f64 = unsafe { msg_send![object, fractionCompleted] };
    state.read_progress.report_progress(Some(completed));
}

static PROGRESS_BRIDGE_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("SNEProgressBridge", superclass).unwrap();

    decl.add_ivar::<*mut c_void>("state");
    decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
    decl.add_method(
        sel!(observeValueForKeyPath:ofObject:change:context:),
        observe_value_for_key_path as extern "C" fn(&mut Object, Sel, id, id, id, *mut c_void),
    );

    decl.register()
});
