use std::{cell::Cell, sync::Mutex, path::{Path, PathBuf}};

use nativeshell_core::{util::Capsule, Context, RunLoopSender};

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
            sender: Context::get().run_loop().new_sender(),
        }
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

pub fn get_target_path(target_folder: &Path, file_name: &str) -> PathBuf {
    let target_path = target_folder.join(&file_name);
    if !target_path.exists() {
        return target_path;
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
            .unwrap_or("".into());
        loop {
            let target_path = target_folder.join(&format!("{} {}{}", stem, i, suffix));
            if !target_path.exists() {
                return target_path;
            }
            i += 1;
        }
    }
}
