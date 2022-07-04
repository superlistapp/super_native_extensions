use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    mem::ManuallyDrop,
    os::raw::c_void,
    rc::{Rc, Weak},
    sync::Arc,
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
    drag_manager::{DragRequest, PendingSourceState, PlatformDragContextDelegate},
    error::{NativeExtensionsError, NativeExtensionsResult},
    util::DropNotifier,
};

use super::{util::to_nsstring, DataSourceSession, DataSourceSessionDelegate, PlatformDataSource};

pub struct PlatformDragContext {
    id: i64,
    weak_self: Late<Weak<Self>>,
    view: StrongPtr,
    delegate: Weak<dyn PlatformDragContextDelegate>,
    interaction: Late<StrongPtr>,
    interaction_delegate: Late<StrongPtr>,
    sessions: RefCell<HashMap<id, Rc<Session>>>,
}

struct Session {
    context_delegate: Weak<dyn PlatformDragContextDelegate>,
    weak_self: Late<Weak<Self>>,
    in_progress: Cell<bool>,
    data_source_session: RefCell<Option<Arc<DataSourceSession>>>,
}

impl Session {
    fn new(context_delegate: Weak<dyn PlatformDragContextDelegate>) -> Self {
        Self {
            context_delegate,
            weak_self: Late::new(),
            in_progress: Cell::new(false),
            data_source_session: RefCell::new(None),
        }
    }

    fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    fn create_items(
        &self,
        source: Rc<PlatformDataSource>,
        source_drop_notifier: Arc<DropNotifier>,
    ) -> id {
        let (items, session) =
            source.create_items(source_drop_notifier.clone(), self.weak_self.clone());
        let mut dragging_items = Vec::<id>::new();
        self.data_source_session.replace(Some(session));
        unsafe {
            for item in items {
                let item_provider = item;
                let drag_item: id = msg_send![class!(UIDragItem), alloc];
                let drag_item: id = msg_send![drag_item, initWithItemProvider: item_provider];
                let drag_item: id = msg_send![drag_item, autorelease];
                dragging_items.push(drag_item);
            }
            NSArray::arrayWithObjects(nil, &dragging_items)
        }
    }

    fn drag_will_begin(&self) {
        self.in_progress.replace(true);
    }
}

impl DataSourceSessionDelegate for Session {
    fn should_fetch_items(&self) -> bool {
        self.in_progress.get()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(session) = self.data_source_session.take() {
            session.dispose();
        }
    }
}

impl PlatformDragContext {
    pub fn new(id: i64, view_handle: i64, delegate: Weak<dyn PlatformDragContextDelegate>) -> Self {
        Self {
            id,
            weak_self: Late::new(),
            view: unsafe { StrongPtr::retain(view_handle as *mut _) },
            delegate,
            interaction: Late::new(),
            interaction_delegate: Late::new(),
            sessions: RefCell::new(HashMap::new()),
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self.clone());
        autoreleasepool(|| unsafe {
            let delegate: id = msg_send![*DELEGATE_CLASS, alloc];
            let delegate: id = msg_send![delegate, init];
            (*delegate).set_ivar("context", Weak::into_raw(weak_self) as *mut c_void);
            self.interaction_delegate.set(StrongPtr::new(delegate));
            let interaction: id = msg_send![class!(UIDragInteraction), alloc];
            let interaction: id = msg_send![interaction, initWithDelegate: delegate];
            self.interaction.set(StrongPtr::new(interaction));
            let () = msg_send![*self.view, addInteraction: interaction];
        });
    }

    pub async fn start_drag(
        &self,
        _request: DragRequest,
        _source: Rc<PlatformDataSource>,
        _notifier: Arc<DropNotifier>,
    ) -> NativeExtensionsResult<()> {
        Err(NativeExtensionsError::UnsupportedOperation)
    }

    fn _items_for_beginning(
        &self,
        _interaction: id,
        drag_session: id,
        source: Rc<PlatformDataSource>,
        source_drop_notifier: Arc<DropNotifier>,
    ) -> id {
        let session = Rc::new(Session::new(self.delegate.clone()));
        session.assign_weak_self(Rc::downgrade(&session));
        self.sessions
            .borrow_mut()
            .insert(drag_session, session.clone());
        session.create_items(source, source_drop_notifier)
    }

    fn items_for_beginning(&self, interaction: id, session: id) -> id {
        if let Some(delegate) = self.delegate.upgrade() {
            let items = delegate.data_source_for_drag_request(self.id, Point { x: 10.0, y: 10.0 });
            loop {
                {
                    let items = items.borrow();
                    match &*items {
                        PendingSourceState::Ok {
                            source,
                            source_drop_notifier: drop_notifier,
                        } => {
                            return self._items_for_beginning(
                                interaction,
                                session,
                                source.clone(),
                                drop_notifier.clone(),
                            );
                        }
                        PendingSourceState::Cancelled => return nil,
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

    fn drag_will_begin(&self, _interaction: id, session: id) {
        let session = self.sessions.borrow().get(&session).cloned();
        if let Some(session) = session {
            session.drag_will_begin();
        }
    }

    fn did_end_with_operation(&self, _interaction: id, session: id, operation: UIDropOperation) {
        if operation == 0 {
            self.sessions.borrow_mut().remove(&session);
        }

        println!("Did end with operation {:?}", operation);
    }

    fn did_transfer_items(&self, _interaction: id, session: id) {
        self.sessions.borrow_mut().remove(&session);
    }
}

impl Drop for PlatformDragContext {
    fn drop(&mut self) {
        unsafe {
            let () = msg_send![*self.view, removeInteraction: **self.interaction];
        }
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

pub unsafe fn superclass(this: &Object) -> &Class {
    let superclass: id = msg_send![this, superclass];
    &*(superclass as *const _)
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

extern "C" fn drag_will_begin(this: &mut Object, _sel: Sel, interaction: id, session: id) {
    with_state(
        this,
        |state| state.drag_will_begin(interaction, session),
        || (),
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

extern "C" fn did_transfer_items(this: &mut Object, _sel: Sel, interaction: id, session: id) {
    with_state(
        this,
        |state| state.did_transfer_items(interaction, session),
        || {},
    );
}

static DELEGATE_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("SNEDragInteractionDelegate", superclass).unwrap();
    decl.add_protocol(Protocol::get("UIDragInteractionDelegate").unwrap());
    decl.add_ivar::<*mut c_void>("context");
    decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
    decl.add_method(
        sel!(dragInteraction:itemsForBeginningSession:),
        items_for_beginning as extern "C" fn(&mut Object, Sel, id, id) -> id,
    );
    decl.add_method(
        sel!(dragInteraction:sessionWillBegin:),
        drag_will_begin as extern "C" fn(&mut Object, Sel, id, id),
    );
    decl.add_method(
        sel!(dragInteraction:session:didEndWithOperation:),
        did_end_with_operation as extern "C" fn(&mut Object, Sel, id, id, UIDropOperation),
    );
    decl.add_method(
        sel!(dragInteraction:sessionDidTransferItems:),
        did_transfer_items as extern "C" fn(&mut Object, Sel, id, id),
    );
    decl.register()
});