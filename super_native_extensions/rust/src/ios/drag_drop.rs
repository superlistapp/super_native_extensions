use std::{
    cell::RefCell,
    mem::ManuallyDrop,
    os::raw::c_void,
    rc::{Rc, Weak},
};

use cocoa::{
    base::{id, nil},
    foundation::{NSArray, NSUInteger},
};
use core_foundation::{runloop::CFRunLoopRunInMode, string::CFStringRef};
use nativeshell_core::util::Late;
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    rc::{autoreleasepool, StrongPtr},
    runtime::{Class, Object, Protocol, Sel},
    sel, sel_impl,
};
use once_cell::sync::Lazy;

use crate::{
    api_model::Point,
    drag_drop_manager::{DragRequest, PendingWriterState, PlatformDragContextDelegate},
    error::{NativeExtensionsError, NativeExtensionsResult},
};

use super::{superclass, util::to_nsstring, PlatformDataSource};

pub struct PlatformDragContext {
    id: i64,
    view: StrongPtr,
    delegate: Weak<dyn PlatformDragContextDelegate>,
    platform_delegate: Late<StrongPtr>,
    current_items: RefCell<Vec<StrongPtr>>,
}

impl PlatformDragContext {
    pub fn new(id: i64, view_handle: i64, delegate: Weak<dyn PlatformDragContextDelegate>) -> Self {
        println!("VIEW {:?}", view_handle);
        Self {
            id,
            view: unsafe { StrongPtr::retain(view_handle as *mut _) },
            delegate,
            platform_delegate: Late::new(),
            current_items: RefCell::new(Vec::new()),
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) -> NativeExtensionsResult<()> {
        autoreleasepool(|| unsafe {
            let delegate: id = msg_send![*DELEGATE_CLASS, alloc];
            let delegate: id = msg_send![delegate, init];
            (*delegate).set_ivar("context", Weak::into_raw(weak_self) as *mut c_void);
            self.platform_delegate.set(StrongPtr::new(delegate));
            let interaction: id = msg_send![class!(UIDragInteraction), alloc];
            let interaction: id =
                msg_send![interaction, initWithDelegate: **self.platform_delegate];
            let interaction: id = msg_send![interaction, autorelease];
            let _: () = msg_send![*self.view, addInteraction: interaction];
        });
        Ok(())
    }

    pub fn register_drop_types(&self, types: &[String]) -> NativeExtensionsResult<()> {
        Ok(())
    }

    pub async fn start_drag(
        &self,
        request: DragRequest,
        source: Rc<PlatformDataSource>,
    ) -> NativeExtensionsResult<()> {
        Err(NativeExtensionsError::UnsupportedOperation)
    }

    fn items_for_beginning(&self, interaction: id, session: id) -> id {
        if let Some(delegate) = self.delegate.upgrade() {
            let items = delegate.writer_for_drag_request(self.id, Point { x: 10.0, y: 10.0 });
            self.current_items.borrow_mut().clear();
            loop {
                {
                    let items = items.borrow();
                    match &*items {
                        PendingWriterState::Ok {
                            source,
                            drop_notifier,
                        } => unsafe {
                            let items = source.create_items(drop_notifier.clone());
                            println!("Items {:?}", items.len());
                            let mut dragging_items = Vec::<id>::new();
                            for item in items {
                                self.current_items
                                    .borrow_mut()
                                    .push(StrongPtr::retain(item));
                                let item_provider: id = msg_send![class!(NSItemProvider), alloc];
                                let item_provider: id =
                                    msg_send![item_provider, initWithObject: item];
                                let item_provider: id = msg_send![item_provider, autorelease];
                                let drag_item: id = msg_send![class!(UIDragItem), alloc];
                                let drag_item: id =
                                    msg_send![drag_item, initWithItemProvider: item_provider];
                                let drag_item: id = msg_send![drag_item, autorelease];
                                dragging_items.push(drag_item);
                            }

                            return NSArray::arrayWithObjects(nil, &dragging_items);
                        },
                        PendingWriterState::Cancelled => return nil,
                        _ => {}
                    }
                }
                let mode = to_nsstring("NativeShellRunLoopMode");
                unsafe { CFRunLoopRunInMode(*mode as CFStringRef, 1.0, 1) };
            }
        } else {
            nil
        }
    }

    fn did_end_with_operation(&self, interaction: id, session: id, operation: UIDropOperation) {
        {
            let mut items = self.current_items.borrow_mut();
            for item in items.iter() {
                let _: () = unsafe { msg_send![**item, disposeState] };
            }
            items.clear();
        }

        println!("Did end with operation {:?}", operation);
    }
}

fn with_state<F, FR, R>(this: id, callback: F, default: FR) -> R
where
    F: FnOnce(Rc<PlatformDragContext>) -> R,
    FR: FnOnce() -> R,
{
    unsafe {
        let context_ptr = {
            let context_ptr: *mut c_void = *(*this).get_ivar("context");
            context_ptr as *const PlatformDragContext
        };
        let this = ManuallyDrop::new(Weak::from_raw(context_ptr));
        let this = this.upgrade();
        match this {
            Some(this) => callback(this),
            None => default(),
        }
    }
}

extern "C" fn dealloc(this: &Object, _sel: Sel) {
    unsafe {
        let context_ptr = {
            let context_ptr: *mut c_void = *this.get_ivar("context");
            context_ptr as *const PlatformDragContext
        };
        Weak::from_raw(context_ptr);

        let superclass = superclass(this);
        let () = msg_send![super(this, superclass), dealloc];
    }
}

extern "C" fn items_for_beginning(
    this: &mut Object,
    _sel: Sel,
    interaction: id,
    session: id,
) -> id {
    with_state(
        this,
        |state| state.items_for_beginning(interaction, session),
        || nil,
    )
}

type UIDropOperation = NSUInteger;

extern "C" fn did_end_with_operation(
    this: &mut Object,
    _sel: Sel,
    interaction: id,
    session: id,
    operation: UIDropOperation,
) {
    with_state(
        this,
        |state| state.did_end_with_operation(interaction, session, operation),
        || {},
    );
}

static DELEGATE_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("IMDragDropInteractionDelegate", superclass).unwrap();
    decl.add_protocol(Protocol::get("UIDragInteractionDelegate").unwrap());
    decl.add_ivar::<*mut c_void>("context");
    decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
    decl.add_method(
        sel!(dragInteraction:itemsForBeginningSession:),
        items_for_beginning as extern "C" fn(&mut Object, Sel, id, id) -> id,
    );
    decl.add_method(
        sel!(dragInteraction:session:didEndWithOperation:),
        did_end_with_operation as extern "C" fn(&mut Object, Sel, id, id, UIDropOperation),
    );
    decl.register()
});
