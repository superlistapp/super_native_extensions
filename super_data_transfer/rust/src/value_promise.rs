use std::sync::{Condvar, Mutex};

use nativeshell_core::{TryFromValue, Value};

#[derive(Debug, TryFromValue, PartialEq)]
#[nativeshell(tag = "type", rename_all = "camelCase")]
pub enum ValuePromiseResult {
    Ok { value: Value },
    Cancelled,
}

pub struct ValuePromise {
    data: Mutex<Option<ValuePromiseResult>>,
    condition: Condvar,
}

#[allow(dead_code)]
impl ValuePromise {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(None),
            condition: Condvar::new(),
        }
    }

    pub fn try_take(&self) -> Option<ValuePromiseResult> {
        let mut lock = self.data.lock().unwrap();
        lock.take()
    }

    pub fn wait(&self) -> ValuePromiseResult {
        let mut lock = self.data.lock().unwrap();
        loop {
            match lock.take() {
                Some(res) => return res,
                None => lock = self.condition.wait(lock).unwrap(),
            }
        }
    }

    pub fn cancel(&self) {
        let mut lock = self.data.lock().unwrap();
        lock.replace(ValuePromiseResult::Cancelled);
        self.condition.notify_one();
    }

    pub fn set(&self, res: ValuePromiseResult) {
        let mut lock = self.data.lock().unwrap();
        lock.replace(res);
        self.condition.notify_one();
    }

    pub fn set_value(&self, value: Value) {
        let mut lock = self.data.lock().unwrap();
        lock.replace(ValuePromiseResult::Ok { value });
        self.condition.notify_one();
    }
}
