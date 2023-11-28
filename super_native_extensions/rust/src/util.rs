use std::{
    cell::Cell,
    ops::Deref,
    path::{Path, PathBuf},
    sync::Mutex,
};

use irondash_run_loop::{util::Capsule, RunLoop, RunLoopSender};

pub struct DropNotifier {
    callback: Mutex<Option<Capsule<Box<dyn FnOnce()>>>>,
    sender: RunLoopSender,
}

impl DropNotifier {
    pub fn new<F: FnOnce() + 'static>(callback: F) -> Self {
        Self::new_with_boxed(Box::new(callback))
    }

    pub fn new_with_boxed(callback: Box<dyn FnOnce()>) -> Self {
        Self {
            callback: Mutex::new(Some(Capsule::new(callback))),
            sender: RunLoop::current().new_sender(),
        }
    }

    pub fn dispose(&self) {
        let callback = self.callback.lock().unwrap().take();

        if let Some(mut callback) = callback {
            if self.sender.is_same_thread() {
                let callback = callback.take().unwrap();
                callback();
            } else {
                self.sender.send(move || {
                    let callback = callback.take().unwrap();
                    callback();
                });
            }
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

#[allow(dead_code)]
pub fn get_target_path(target_folder: &Path, file_name: &str) -> PathBuf {
    let target_path = target_folder.join(file_name);
    if !target_path.exists() {
        target_path
    } else {
        let mut i = 2;
        let source_path = Path::new(file_name);
        let stem = source_path
            .file_stem()
            .expect("Couldn't get file stem")
            .to_string_lossy();
        let extension = source_path.extension();
        let suffix = extension
            .map(|a| format!(".{}", a.to_string_lossy()))
            .unwrap_or_else(|| "".into());
        loop {
            let target_path = target_folder.join(format!("{stem} {i}{suffix}"));
            if !target_path.exists() {
                return target_path;
            }
            i += 1;
        }
    }
}

/// Structure that can be used to move non-send objects accross thread. Unsafe.
pub struct Movable<T>(T);
unsafe impl<T> Send for Movable<T> {}

#[allow(dead_code)]
impl<T> Movable<T> {
    /// Safety: This function is unsafe because it turns non Send object into Send.
    pub unsafe fn new(t: T) -> Self {
        Self(t)
    }

    pub fn take(self) -> T {
        self.0
    }
}

impl<T> Deref for Movable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Clone> Clone for Movable<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

//
//
//

pub trait TryGetOrInsert<T> {
    fn try_get_or_insert_with<E, F>(&mut self, f: F) -> Result<&mut T, E>
    where
        F: FnOnce() -> Result<T, E>;
}

impl<T> TryGetOrInsert<T> for Option<T> {
    fn try_get_or_insert_with<E, F>(&mut self, f: F) -> Result<&mut T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        match self {
            Some(value) => Ok(value),
            None => Ok(self.get_or_insert(f()?)),
        }
    }
}
