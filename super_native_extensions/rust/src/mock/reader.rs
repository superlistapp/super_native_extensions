use std::{cell::RefCell, rc::Weak};

use nativeshell_core::{util::Late, Value};

use crate::error::NativeExtensionsResult;

thread_local! {
    pub static READERS: RefCell<Vec<Weak<PlatformClipboardReader>>> = RefCell::new(Vec::new());
}

pub struct PlatformClipboardReader {
    weak_self: Late<Weak<Self>>,
}

impl PlatformClipboardReader {
    pub async fn get_items(&self) -> NativeExtensionsResult<Vec<i64>> {
        Ok(vec![])
    }

    pub async fn get_types_for_item(&self, _item: i64) -> NativeExtensionsResult<Vec<String>> {
        Ok(vec![])
    }

    pub async fn get_data_for_item(
        &self,
        _item: i64,
        _data_type: String,
    ) -> NativeExtensionsResult<Value> {
        Ok(Value::Null)
    }

    pub fn new_default() -> NativeExtensionsResult<Self> {
        Ok(Self {
            weak_self: Late::new(),
        })
    }

    pub fn assign_weak_self(&self, weak_self: Weak<PlatformClipboardReader>) {
        self.weak_self.set(weak_self.clone());
        READERS.with(|f| {
            f.borrow_mut().push(weak_self);
        })
    }
}

impl Drop for PlatformClipboardReader {
    fn drop(&mut self) {
        READERS.with(|f| {
            f.borrow_mut()
                .retain(|a| a.as_ptr() != self.weak_self.as_ptr());
        });
    }
}
