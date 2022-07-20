use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    mem::ManuallyDrop,
    os::raw::c_void,
    rc::{Rc, Weak},
};

use cocoa::{
    base::{id, nil, BOOL, NO, YES},
    foundation::NSArray,
};
use core_graphics::geometry::CGPoint;

use nativeshell_core::{util::Late, Context, Value};
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
    api_model::{DataProviderId, DragConfiguration, DragRequest, DropOperation, Point},
    drag_manager::{
        DataProviderEntry, DragSessionId, PendingSourceData, PendingSourceState,
        PlatformDragContextDelegate,
    },
    error::{NativeExtensionsError, NativeExtensionsResult},
    platform_impl::platform::os::drag_common::{UIDropOperationCancel, UIDropOperationForbidden},
};

use super::{
    drag_common::{DropOperationExt, UIDropOperation},
    DataProviderSessionDelegate,
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
    configuration: DragConfiguration,
    data_providers: RefCell<HashMap<DataProviderId, DataProviderEntry>>,
}

impl Session {
    fn new(
        context_delegate: Weak<dyn PlatformDragContextDelegate>,
        context_id: i64,
        session_id: DragSessionId,
        configuration: DragConfiguration,
    ) -> Self {
        Self {
            context_delegate,
            context_id,
            weak_self: Late::new(),
            in_progress: Cell::new(false),
            session_id,
            configuration,
            data_providers: RefCell::new(HashMap::new()),
        }
    }

    fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    fn create_items(
        &self,
        provider_ids: Vec<DataProviderId>,
        mut providers: HashMap<DataProviderId, DataProviderEntry>,
    ) -> id {
        let mut dragging_items = Vec::<id>::new();
        unsafe {
            for provider_id in provider_ids {
                let provider_entry = providers.remove(&provider_id).expect("Missing provider");
                // We manage the data source notifier ourselves. Unfortunately the
                // NSItemProvider leaks and never gets released on iOS.
                // So after dragging is finished we manually drop the notifier releasing
                // everything data-source related. The DataSourceSession will be kept
                // alive but it only has weak references to PlatformDataProvider and
                // PlatformDataProviderState.
                let item_provider = provider_entry
                    .provider
                    .create_ns_item_provider(None, Some(self.weak_self.clone()));
                // Keep providers alive
                self.data_providers
                    .borrow_mut()
                    .insert(provider_id, provider_entry);
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

    fn did_move(&self, location: Point) {
        if let Some(delegate) = self.context_delegate.upgrade() {
            delegate.drag_session_did_move_to_location(self.context_id, self.session_id, location);
        }
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

impl DataProviderSessionDelegate for Session {
    fn should_fetch_items(&self) -> bool {
        self.in_progress.get()
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
        _providers: HashMap<DataProviderId, DataProviderEntry>,
        _session_id: DragSessionId,
    ) -> NativeExtensionsResult<()> {
        Err(NativeExtensionsError::UnsupportedOperation)
    }

    fn _items_for_beginning(
        &self,
        _interaction: id,
        drag_session: id,
        data: PendingSourceData,
    ) -> id {
        let provider_ids: Vec<_> = data
            .configuration
            .items
            .iter()
            .map(|i| i.data_provider_id)
            .collect();
        let session = Rc::new(Session::new(
            self.delegate.clone(),
            self.id,
            data.session_id,
            data.configuration,
        ));
        session.assign_weak_self(Rc::downgrade(&session));
        self.sessions
            .borrow_mut()
            .insert(drag_session, session.clone());
        session.create_items(provider_ids, data.providers)
    }

    fn items_for_beginning(&self, interaction: id, session: id) -> id {
        if let Some(delegate) = self.delegate.upgrade() {
            let data_source =
                delegate.get_data_for_drag_request(self.id, Point { x: 10.0, y: 10.0 });
            loop {
                {
                    let items = data_source.replace(PendingSourceState::Pending);
                    match items {
                        PendingSourceState::Ok(data) => {
                            return self._items_for_beginning(interaction, session, data);
                        }
                        PendingSourceState::Cancelled => return nil,
                        _ => {}
                    }
                }
                Context::get().run_loop().platform_run_loop.poll_once();
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

    fn did_move(&self, _interaction: id, session: id) {
        let location: CGPoint = unsafe { msg_send![session, locationInView:*self.view] };
        let session = self.sessions.borrow().get(&session).cloned();
        if let Some(session) = session {
            session.did_move(location.into());
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
                .configuration
                .allowed_operations
                .contains(&DropOperation::Move)
        } else {
            false
        }
    }

    fn did_transfer_items(&self, _interaction: id, session: id) {
        self.sessions.borrow_mut().remove(&session);
    }

    pub fn get_local_data(&self, session: id) -> Value {
        // let sessions = self.sessions.borrow();
        // if let Some(session) = sessions.get(&session) {
        //     session.configuration.local_data.clone()
        // } else {
        Value::Null
        // }
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

extern "C" fn did_move(this: &mut Object, _sel: Sel, interaction: id, session: id) {
    with_state(this, |state| state.did_move(interaction, session), || ())
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
        sel!(dragInteraction:sessionDidMove:),
        did_move as extern "C" fn(&mut Object, Sel, id, id),
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
