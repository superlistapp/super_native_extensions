use std::{
    cell::Cell,
    sync::{Arc, Mutex},
};

use nativeshell_core::{util::Capsule, Context, RunLoopSender};

pub struct DropNotifier {
    callback: Mutex<Option<Capsule<Box<dyn FnOnce()>>>>,
    sender: RunLoopSender,
}

impl DropNotifier {
    pub fn new<F: FnOnce() + 'static>(callback: F) -> Arc<Self> {
        Self::new_with_boxed(Box::new(callback))
    }

    pub fn new_with_boxed(callback: Box<dyn FnOnce()>) -> Arc<Self> {
        Arc::new(Self {
            callback: Mutex::new(Some(Capsule::new(callback))),
            sender: Context::get().run_loop().new_sender(),
        })
    }

    pub fn new_<F: FnOnce() + 'static>(callback: F) -> Self {
        Self::new_with_boxed_(Box::new(callback))
    }

    pub fn new_with_boxed_(callback: Box<dyn FnOnce()>) -> Self {
        Self {
            callback: Mutex::new(Some(Capsule::new(callback))),
            sender: Context::get().run_loop().new_sender(),
        }
    }

    pub fn new_combined(notifiers: &[Arc<DropNotifier>]) -> Arc<Self> {
        let notifiers: Vec<Arc<DropNotifier>> = notifiers.into();
        DropNotifier::new(move || {
            let _notifiers = notifiers;
        })
    }

    pub fn dispose(&self) {
        let callback = self.callback.lock().unwrap().take();

        if let Some(mut callback) = callback {
            self.sender.send(move || {
                let callback = callback.take().unwrap();
                callback();
            });
        }
    }
}

impl Drop for DropNotifier {
    fn drop(&mut self) {
        self.dispose();
    }
}

pub trait NextId {
    fn next_id(&self) -> i64;
}

impl NextId for Cell<i64> {
    fn next_id(&self) -> i64 {
        let next_id = self.get();
        self.replace(next_id + 1);
        next_id
    }
}
