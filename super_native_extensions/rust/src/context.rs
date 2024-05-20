use std::{
    any::{Any, TypeId},
    cell::{Ref, RefCell},
    collections::HashMap,
    rc::Rc,
};

pub struct Context {
    internal: Rc<ContextInternal>,
    outermost: bool,
}

#[cfg(not(target_os = "windows"))]
struct Initializer {}

#[cfg(not(target_os = "windows"))]
impl Initializer {
    fn new() -> Self {
        Self {}
    }
}

#[cfg(target_os = "windows")]
type Initializer = crate::platform_impl::platform::OleInitializer;

pub struct ContextInternal {
    attachments: RefCell<HashMap<TypeId, (Box<dyn Any>, usize /* insertion order */)>>,
    _initializer: Initializer,
}

impl Context {
    /// Creates a new context. The will be be associated with current thread and can
    /// be retrieved at any point while the instance is in scope by [`Context::get()`].
    ///
    /// Any NativeShell application must have exactly one context active.
    pub fn new() -> Self {
        let internal = Rc::new(ContextInternal {
            attachments: RefCell::new(HashMap::new()),
            _initializer: Initializer::new(),
        });
        let res = Self {
            internal: internal.clone(),
            outermost: true,
        };
        let res_fallback = Self {
            internal,
            outermost: true,
        };

        let result = res.clone();
        let prev_context = CURRENT_CONTEXT.with(|c| c.replace(Some(res)));
        if prev_context.is_some() {
            panic!("another context is already associated with current thread.");
        }
        CURRENT_CONTEXT_FALLBACK.with(|c| c.replace(Some(res_fallback)));
        result
    }

    pub fn get_attachment<T: Any, F: FnOnce() -> T>(&self, on_init: F) -> Ref<T> {
        let id = TypeId::of::<T>();
        // Do a separate check here, make sure attachments is not borrowed while
        // creating the attachment
        if !self.internal.attachments.borrow().contains_key(&id) {
            let attachment = Box::new(on_init());
            // store len to preserve insertion order; This will be used when dropping
            let len = self.internal.attachments.borrow().len();
            self.internal
                .attachments
                .borrow_mut()
                .insert(id, (attachment, len));
        }
        let map = self.internal.attachments.borrow();
        Ref::map(map, |r| {
            let any = r.get(&id).unwrap();
            any.0.downcast_ref::<T>().unwrap()
        })
    }

    /// Returns context associated with current thread. Can only be called
    /// on main thread and only while the original (outer-most) context is
    /// still in scope. Otherwise the function will panic.
    pub fn get() -> Self {
        Self::current().expect("no context is associated with current thread.")
    }

    /// Returns context associated with current thread.
    pub fn current() -> Option<Self> {
        // It is necessary to be able to access Context::current() while thread locals
        // are being destructed in case attachment destructor wants to access context.
        // Which ever thread local is removed first will clean the attachment.
        let res = CURRENT_CONTEXT.try_with(|c| c.borrow().as_ref().map(|c| c.clone()));
        match res {
            Ok(res) => res,
            // (Can happen if attachment accesses context during destruction.)
            // CURRENT_CONTEXT is being destroyed, use the fallback; Reverse situation
            // can not happen because attachments will be removed by CURRENT_CONTEXT_FALLBACK
            // destructor.
            Err(_) => CURRENT_CONTEXT_FALLBACK.with(|c| c.borrow().as_ref().map(|c| c.clone())),
        }
    }
}

thread_local! {
    static CURRENT_CONTEXT: RefCell<Option<Context>> = const { RefCell::new(None) };
    static CURRENT_CONTEXT_FALLBACK: RefCell<Option<Context>> = const { RefCell::new(None) };
}

impl Drop for Context {
    fn drop(&mut self) {
        if self.outermost {
            // Remove attachment in reverse order in which they were inserted
            while self.internal.attachments.borrow().len() > 0 {
                let to_remove_index = self.internal.attachments.borrow().len() - 1;
                let to_remove = self
                    .internal
                    .attachments
                    .borrow()
                    .iter()
                    .find(|e| e.1 .1 == to_remove_index)
                    .map(|a| *a.0)
                    .expect("Attachment to remove not found");

                // Hold removed item until RefMut gets dropped.
                let _removed = { self.internal.attachments.borrow_mut().remove(&to_remove) };

                if to_remove_index == 0 {
                    break;
                }
            }
            CURRENT_CONTEXT.try_with(|c| c.take()).ok();
            CURRENT_CONTEXT_FALLBACK.try_with(|c| c.take()).ok();
        }
    }
}

impl Context {
    // Intentionally private
    fn clone(&self) -> Context {
        Context {
            internal: self.internal.clone(),
            outermost: false,
        }
    }
}
