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

use nativeshell_core::{util::Late, Context};
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
    api_model::DropOperation,
    drop_manager::{BaseDropEvent, DropEvent, PlatformDropContextDelegate},
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    platform_impl::platform::common::from_nsstring,
};

use super::{drag_common::DropOperationExt, superclass, PlatformDataReader};

pub struct PlatformDropContext {
    id: i64,
    weak_self: Late<Weak<Self>>,
    view: StrongPtr,
    delegate: Weak<dyn PlatformDropContextDelegate>,
    interaction: Late<StrongPtr>,
    interaction_delegate: Late<StrongPtr>,
    sessions: RefCell<HashMap<id, Rc<Session>>>,
}

struct Session {
    last_operation: Cell<DropOperation>,
}

impl PlatformDropContext {
    pub fn new(id: i64, view_handle: i64, delegate: Weak<dyn PlatformDropContextDelegate>) -> Self {
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

    pub fn register_drop_types(&self, _types: &[String]) -> NativeExtensionsResult<()> {
        Ok(())
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self.clone());
        autoreleasepool(|| unsafe {
            let delegate: id = msg_send![*DELEGATE_CLASS, new];
            (*delegate).set_ivar("context", Weak::into_raw(weak_self) as *mut c_void);
            self.interaction_delegate.set(StrongPtr::new(delegate));
            let interaction: id = msg_send![class!(UIDropInteraction), alloc];
            let interaction: id = msg_send![interaction, initWithDelegate: delegate];
            self.interaction.set(StrongPtr::new(interaction));
            let () = msg_send![*self.view, addInteraction: interaction];
        });
    }

    fn get_session(&self, session: id) -> Rc<Session> {
        self.sessions
            .borrow_mut()
            .entry(session)
            .or_insert_with(|| {
                Rc::new(Session {
                    last_operation: Cell::new(DropOperation::None),
                })
            })
            .clone()
    }

    fn delegate(&self) -> NativeExtensionsResult<Rc<dyn PlatformDropContextDelegate>> {
        self.delegate
            .upgrade()
            .ok_or_else(|| NativeExtensionsError::OtherError("no delegate".into()))
    }

    fn create_drop_event(
        &self,
        session: id,
        is_perform_drop: bool,
    ) -> NativeExtensionsResult<DropEvent> {
        let delegate = self.delegate()?;

        let location: CGPoint = unsafe { msg_send![session, locationInView: *self.view] };
        let allows_move: BOOL = unsafe { msg_send![session, allowsMoveOperation] };
        let allowed_operations = if allows_move == YES {
            vec![DropOperation::Copy, DropOperation::Move]
        } else {
            vec![DropOperation::Copy]
        };

        // local data
        let local_session: id = unsafe { msg_send![session, localDragSession] };
        let local_data = delegate
            .get_platform_drag_context(self.id)?
            .get_local_data(local_session);

        // formats
        let mut formats = Vec::<String>::new();
        let items: id = unsafe { msg_send![session, items] };
        for i in 0..unsafe { NSArray::count(items) } {
            let item: id = unsafe { NSArray::objectAtIndex(items, i) };
            let item_provider: id = unsafe { msg_send![item, itemProvider] };
            let identifiers: id = unsafe { msg_send![item_provider, registeredTypeIdentifiers] };
            for j in 0..unsafe { NSArray::count(identifiers) } {
                let identifier = unsafe { from_nsstring(NSArray::objectAtIndex(identifiers, j)) };
                if !formats.contains(&identifier) {
                    formats.push(identifier);
                }
            }
        }

        let accepted_operation = if is_perform_drop {
            Some(self.get_session(session).last_operation.get())
        } else {
            None
        };

        let reader = if is_perform_drop {
            let platform_reader = PlatformDataReader::new_with_drop_session_items(items)?;
            Some(delegate.register_platform_reader(platform_reader)?)
        } else {
            None
        };

        Ok(DropEvent {
            session_id: session as i64,
            location_in_view: location.into(),
            local_data,
            allowed_operations,
            formats,
            accepted_operation,
            reader,
        })
    }

    fn session_did_update(&self, session: id) -> NativeExtensionsResult<id> {
        let delegate = self.delegate()?;
        let event = self.create_drop_event(session, false)?;
        let allows_move: BOOL = unsafe { msg_send![session, allowsMoveOperation] };

        let session = self.get_session(session);
        let session_clone = session.clone();
        delegate.send_drop_update(
            self.id,
            event,
            Box::new(move |res| {
                let mut res = res.ok_log().unwrap_or(DropOperation::None);
                if res == DropOperation::Move && allows_move == NO {
                    res = DropOperation::Copy;
                }
                session_clone.last_operation.replace(res);
            }),
        );

        let operation = session.last_operation.get().to_platform();

        let proposal: id = unsafe { msg_send![class!(UIDropProposal), alloc] };
        let proposal: id = unsafe { msg_send![proposal, initWithDropOperation: operation] };
        let () = unsafe { msg_send![proposal, autorelease] };

        Ok(proposal)
    }

    fn perform_drop(&self, session: id) -> NativeExtensionsResult<()> {
        let event = self.create_drop_event(session, true)?;
        let delegate = self.delegate()?;
        let done = Rc::new(Cell::new(false));
        let done_clone = done.clone();
        delegate.send_perform_drop(
            self.id,
            event,
            Box::new(move |r| {
                r.ok_log();
                done_clone.set(true);
            }),
        );
        while !done.get() {
            Context::get().run_loop().platform_run_loop.poll_once();
        }
        Ok(())
    }

    fn session_did_exit(&self, session: id) -> NativeExtensionsResult<()> {
        if let Some(delegate) = self.delegate.upgrade() {
            delegate.send_drop_leave(
                self.id,
                BaseDropEvent {
                    session_id: session as i64,
                },
            );
        }
        Ok(())
    }

    fn session_did_end(&self, session: id) -> NativeExtensionsResult<()> {
        if let Some(delegate) = self.delegate.upgrade() {
            delegate.send_drop_ended(
                self.id,
                BaseDropEvent {
                    session_id: session as i64,
                },
            );
        }
        self.sessions.borrow_mut().remove(&session);
        Ok(())
    }
}

impl Drop for PlatformDropContext {
    fn drop(&mut self) {
        unsafe {
            let () = msg_send![*self.view, removeInteraction: **self.interaction];
        }
    }
}

fn with_state<F, FR, R>(this: id, callback: F, default: FR) -> R
where
    F: FnOnce(Rc<PlatformDropContext>) -> R,
    FR: FnOnce() -> R,
{
    unsafe {
        let context_ptr = {
            let context_ptr: *mut c_void = *(*this).get_ivar("context");
            context_ptr as *const PlatformDropContext
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
            context_ptr as *const PlatformDropContext
        };
        Weak::from_raw(context_ptr);

        let superclass = superclass(this);
        let () = msg_send![super(this, superclass), dealloc];
    }
}

extern "C" fn can_handle_session(
    _this: &Object,
    _sel: Sel,
    _interaction: id,
    _session: id,
) -> BOOL {
    YES
}

extern "C" fn session_did_update(
    this: &mut Object,
    _sel: Sel,
    _interaction: id,
    session: id,
) -> id {
    with_state(
        this,
        |state| state.session_did_update(session).ok_log().unwrap_or(nil),
        || nil,
    )
}

extern "C" fn perform_drop(this: &mut Object, _sel: Sel, _interaction: id, session: id) {
    with_state(
        this,
        |state| state.perform_drop(session).ok_log().unwrap_or(()),
        || (),
    )
}

extern "C" fn session_did_exit(this: &mut Object, _sel: Sel, _interaction: id, session: id) {
    with_state(
        this,
        |state| state.session_did_exit(session).ok_log().unwrap_or(()),
        || (),
    )
}

extern "C" fn session_did_end(this: &mut Object, _sel: Sel, _interaction: id, session: id) {
    with_state(
        this,
        |state| state.session_did_end(session).ok_log().unwrap_or(()),
        || (),
    )
}

static DELEGATE_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("SNEDropInteractionDelegate", superclass).unwrap();
    decl.add_protocol(Protocol::get("UIDropInteractionDelegate").unwrap());
    decl.add_ivar::<*mut c_void>("context");
    decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
    decl.add_method(
        sel!(dropInteraction:canHandleSession:),
        can_handle_session as extern "C" fn(&Object, Sel, id, id) -> BOOL,
    );
    decl.add_method(
        sel!(dropInteraction:sessionDidUpdate:),
        session_did_update as extern "C" fn(&mut Object, Sel, id, id) -> id,
    );
    decl.add_method(
        sel!(dropInteraction:performDrop:),
        perform_drop as extern "C" fn(&mut Object, Sel, id, id),
    );
    decl.add_method(
        sel!(dropInteraction:sessionDidExit:),
        session_did_exit as extern "C" fn(&mut Object, Sel, id, id),
    );
    decl.add_method(
        sel!(dropInteraction:sessionDidEnd:),
        session_did_end as extern "C" fn(&mut Object, Sel, id, id),
    );
    decl.register()
});
