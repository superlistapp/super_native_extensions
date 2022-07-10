use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use cocoa::{
    base::{id, nil},
    foundation::NSArray,
};
use nativeshell_core::util::Late;
use objc::{
    msg_send,
    rc::{autoreleasepool, StrongPtr},
    runtime::{Object, Sel},
    sel, sel_impl,
};

use crate::{
    drop_manager::PlatformDropContextDelegate, error::NativeExtensionsResult,
    platform_impl::platform::common::to_nsstring,
};

use super::{
    drag_common::{NSDragOperation, NSDragOperationCopy, NSDragOperationNone},
    util::class_decl_from_name,
};

pub struct PlatformDropContext {
    id: i64,
    weak_self: Late<Weak<Self>>,
    view: StrongPtr,
    delegate: Weak<dyn PlatformDropContextDelegate>,
}

static ONCE: std::sync::Once = std::sync::Once::new();

thread_local! {
    pub static VIEW_TO_CONTEXT: RefCell<HashMap<id, Weak<PlatformDropContext>>> = RefCell::new(HashMap::new());
}

impl PlatformDropContext {
    pub fn new(id: i64, view_handle: i64, delegate: Weak<dyn PlatformDropContextDelegate>) -> Self {
        ONCE.call_once(prepare_flutter);
        Self {
            id,
            weak_self: Late::new(),
            view: unsafe { StrongPtr::retain(view_handle as *mut _) },
            delegate,
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        VIEW_TO_CONTEXT.with(|v| {
            v.borrow_mut().insert(*self.view, weak_self.clone());
        });
        self.weak_self.set(weak_self);
    }

    pub fn register_drop_types(&self, types: &[String]) -> NativeExtensionsResult<()> {
        autoreleasepool(|| unsafe {
            let types: Vec<id> = types
                .iter()
                .map(|ty| to_nsstring(&ty).autorelease())
                .collect();
            let types = NSArray::arrayWithObjects(nil, &types);
            let _: id = msg_send![*self.view, registerForDraggedTypes: types];
        });
        Ok(())
    }

    fn dragging_updated(&self, event: id) -> NSDragOperation {
        unsafe {
            // println!("Draaag {:?}", from_nsstring(msg_send![event, description]));
        }
        NSDragOperationCopy
    }
}

impl Drop for PlatformDropContext {
    fn drop(&mut self) {
        VIEW_TO_CONTEXT.with(|v| {
            v.borrow_mut().remove(&*self.view);
        });
    }
}

fn with_state<F, FR, R>(this: id, callback: F, default: FR) -> R
where
    F: FnOnce(Rc<PlatformDropContext>) -> R,
    FR: FnOnce() -> R,
{
    let state = VIEW_TO_CONTEXT
        .with(|v| v.borrow().get(&this).cloned())
        .and_then(|a| a.upgrade());
    if let Some(state) = state {
        callback(state)
    } else {
        default()
    }
}

fn prepare_flutter() {
    unsafe {
        let mut class = class_decl_from_name("FlutterView");

        class.add_method(
            sel!(draggingEntered:),
            dragging_updated as extern "C" fn(&mut Object, Sel, id) -> NSDragOperation,
        );

        class.add_method(
            sel!(draggingUpdated:),
            dragging_updated as extern "C" fn(&mut Object, Sel, id) -> NSDragOperation,
        );
    }
}

extern "C" fn dragging_updated(this: &mut Object, _: Sel, event: id) -> NSDragOperation {
    with_state(
        this,
        |state| state.dragging_updated(event),
        || NSDragOperationNone,
    )
}
