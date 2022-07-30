use std::{
    path::PathBuf,
    rc::{Rc, Weak},
    sync::Arc,
};

use gdk::{Atom, Display};
use gtk::Clipboard;
use nativeshell_core::{util::Late, Value};

use crate::{
    error::{NativeExtensionsError, NativeExtensionsResult},
    reader_manager::ReadProgress,
};

use super::{
    clipboard_async::ClipboardAsync,
    common::{target_includes_text, TYPE_URI},
};

pub struct PlatformDataReader {
    clipboard: Clipboard,
    inner: Late<Inner>,
}

struct Inner {
    targets: Vec<String>,
    uris: Vec<String>,
}

impl PlatformDataReader {
    async fn init(&self) {
        if !self.inner.is_set() {
            let targets = self.clipboard.get_targets().await;
            let uris = if targets.iter().any(|t| t == TYPE_URI) {
                self.clipboard.get_uri_list().await
            } else {
                Vec::new()
            };
            // double check - we might have been preemted
            if !self.inner.is_set() {
                self.inner.set(Inner { targets, uris })
            }
        }
    }

    pub async fn get_items(&self) -> NativeExtensionsResult<Vec<i64>> {
        self.init().await;
        // uris from urilist are represented as separate items
        let num_items = 1.max(self.inner.uris.len());
        Ok((0..num_items as i64).collect())
    }

    pub async fn get_formats_for_item(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        self.init().await;
        if item == 0 {
            Ok(self.inner.targets.clone())
        } else if (item as usize) < self.inner.uris.len() {
            Ok(vec![TYPE_URI.into()])
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn get_data_for_item(
        &self,
        item: i64,
        data_type: String,
        _progress: Arc<ReadProgress>,
    ) -> NativeExtensionsResult<Value> {
        let item = item as usize;
        if data_type == TYPE_URI && item < self.inner.uris.len() {
            Ok(self.inner.uris[item].clone().into())
        } else if item == 0 {
            let target = Atom::intern(&data_type);
            let is_text = target_includes_text(&target);
            if is_text {
                Ok(self.clipboard.get_text().await.into())
            } else {
                Ok(self.clipboard.get_data(&data_type).await.into())
            }
        } else {
            Ok(Value::Null)
        }
    }

    pub fn new_clipboard_reader() -> NativeExtensionsResult<Rc<Self>> {
        let display = Display::default()
            .ok_or_else(|| NativeExtensionsError::OtherError("Display not found".into()))?;
        let clipboard = Clipboard::default(&display)
            .ok_or_else(|| NativeExtensionsError::OtherError("Clipboard not found".into()))?;
        let res = Rc::new(PlatformDataReader {
            clipboard,
            inner: Late::new(),
        });
        res.assign_weak_self(Rc::downgrade(&res));
        Ok(res)
    }

    pub fn assign_weak_self(&self, _weak: Weak<PlatformDataReader>) {}

    pub async fn can_get_virtual_file_for_item(
        &self,
        _item: i64,
        _format: &str,
    ) -> NativeExtensionsResult<bool> {
        Ok(false)
    }

    pub async fn get_virtual_file_for_item(
        &self,
        _item: i64,
        _format: &str,
        _target_folder: PathBuf,
        _progress: Arc<ReadProgress>,
    ) -> NativeExtensionsResult<PathBuf> {
        Err(NativeExtensionsError::UnsupportedOperation)
    }
}
