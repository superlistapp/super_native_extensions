use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    mem::ManuallyDrop,
    os::raw::c_void,
    rc::{Rc, Weak},
    sync::Arc,
};

use cocoa::{
    base::{id, nil, BOOL, NO, YES},
    foundation::NSArray,
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
    api_model::{DataSource, DragConfiguration, DragRequest, DropOperation, Point},
    drag_manager::{DragSessionId, PendingSourceState, PlatformDragContextDelegate},
    error::{NativeExtensionsError, NativeExtensionsResult},
    platform_impl::platform::{
        common::to_nsstring,
        os::drag_common::{UIDropOperationCancel, UIDropOperationForbidden},
    },
    util::DropNotifier,
};

use super::{
    drag_common::{DropOperationExt, UIDropOperation},
    DataSourceSessionDelegate, PlatformDataSource,
};

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
    context_id: i64,
    session_id: DragSessionId,
    weak_self: Late<Weak<Self>>,
    in_progress: Cell<bool>,
    data_source_notifier: RefCell<Option<Arc<DropNotifier>>>,
    drag_data: DragConfiguration,
}

impl Session {
    fn new(
        context_delegate: Weak<dyn PlatformDragContextDelegate>,
        context_id: i64,
        session_id: DragSessionId,
        data_source: DragConfiguration,
    ) -> Self {
        Self {
            context_delegate,
            context_id,
            weak_self: Late::new(),
            in_progress: Cell::new(false),
            data_source_notifier: RefCell::new(None),
            session_id,
            drag_data: data_source,
        }
    }

    fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    fn create_items(
        &self,
        source: Rc<PlatformDataSource>,
        data_source_notifier: Arc<DropNotifier>,
    ) -> id {
        // We manage the data source notifier ourselves. Unfortunately the
        // NSItemProvider leaks and never gets released on iOS.
        // So after dragging is finished we manually drop the notifier releasing
        // everything data-source related. The DataSourceSession will be kept
        // alive but it only has weak references to PlatformDataSource and
        // PlatformDataSourceState.
        self.data_source_notifier
            .replace(Some(data_source_notifier));
        let items = source.create_items(None, self.weak_self.clone());
        let mut dragging_items = Vec::<id>::new();
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

    fn did_end_with_operation(&self, operation: UIDropOperation) {
        if let Some(delegate) = self.context_delegate.upgrade() {
            delegate.drag_session_did_end_with_operation(
                self.context_id,
                self.session_id,
                DropOperation::from_platform(operation),
            );
        }
    }
}

impl DataSourceSessionDelegate for Session {
    fn should_fetch_items(&self) -> bool {
        self.in_progress.get()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(notifier) = self.data_source_notifier.take() {
            notifier.dispose();
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
        _session_id: DragSessionId,
    ) -> NativeExtensionsResult<()> {
        Err(NativeExtensionsError::UnsupportedOperation)
    }

    fn _items_for_beginning(
        &self,
        _interaction: id,
        drag_session: id,
        source: Rc<PlatformDataSource>,
        source_drop_notifier: Arc<DropNotifier>,
        session_id: DragSessionId,
        drag_data: DragConfiguration,
    ) -> id {
        let session = Rc::new(Session::new(
            self.delegate.clone(),
            self.id,
            session_id,
            drag_data,
        ));
        session.assign_weak_self(Rc::downgrade(&session));
        self.sessions
            .borrow_mut()
            .insert(drag_session, session.clone());
        session.create_items(source, source_drop_notifier)
    }

    fn items_for_beginning(&self, interaction: id, session: id) -> id {
        if let Some(delegate) = self.delegate.upgrade() {
            let data_source =
                delegate.get_data_for_drag_request(self.id, Point { x: 10.0, y: 10.0 });
            loop {
                {
                    let items = data_source.replace(PendingSourceState::Pending);
                    match items {
                        PendingSourceState::Ok {
                            source,
                            source_drop_notifier: drop_notifier,
                            session_id,
                            drag_data,
                        } => {
                            return self._items_for_beginning(
                                interaction,
                                session,
                                source.clone(),
                                drop_notifier.clone(),
                                session_id,
                                drag_data,
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
        if let Some(session) = self.sessions.borrow().get(&session).cloned() {
            session.did_end_with_operation(operation);
        }
        // If drop failed remove session here, otherwise we'll do it in did_transfer_items
        if operation == UIDropOperationCancel || operation == UIDropOperationForbidden {
            self.sessions.borrow_mut().remove(&session);
        }
    }

    fn allows_move_operation(&self, _interaction: id, session: id) -> bool {
        if let Some(session) = self.sessions.borrow().get(&session).cloned() {
            session
                .drag_data
                .allowed_operations
                .contains(&DropOperation::Move)
        } else {
            false
        }
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

extern "C" fn allows_move_operation(
    this: &mut Object,
    _sel: Sel,
    interaction: id,
    session: id,
) -> BOOL {
    with_state(
        this,
        |state| {
            if state.allows_move_operation(interaction, session) {
                YES
            } else {
                NO
            }
        },
        || NO,
    )
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
        sel!(dragInteraction:sessionAllowsMoveOperation:),
        allows_move_operation as extern "C" fn(&mut Object, Sel, id, id) -> BOOL,
    );
    decl.add_method(
        sel!(dragInteraction:sessionDidTransferItems:),
        did_transfer_items as extern "C" fn(&mut Object, Sel, id, id),
    );
    decl.register()
});
