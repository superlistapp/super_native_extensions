use std::sync::{Arc, Mutex};

use nativeshell_core::{util::Capsule, Context, RunLoopSender};

pub struct DropNotifier {
    callback: Mutex<Option<Capsule<Box<dyn FnOnce()>>>>,
    sender: RunLoopSender,
}

impl DropNotifier {
    pub fn new<F: FnOnce() + 'static>(callback: F) -> Arc<Self> {
        Arc::new(Self {
            callback: Mutex::new(Some(Capsule::new(Box::new(callback)))),
            sender: Context::get().run_loop().new_sender(),
        })
    }
}

impl Drop for DropNotifier {
    fn drop(&mut self) {
        let mut callback = self
            .callback
            .lock()
            .unwrap()
            .take()
            .expect("callback already called");
        self.sender.send(move || {
            let callback = callback.take().unwrap();
            callback();
        });
    }
}
