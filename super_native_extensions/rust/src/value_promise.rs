use std::sync::{Condvar, Mutex};

use irondash_message_channel::{TryFromValue, Value};

pub struct Promise<T> {
    data: Mutex<Option<T>>,
    condition: Condvar,
}

#[allow(dead_code)]
impl<T> Promise<T> {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(None),
            condition: Condvar::new(),
        }
    }

    pub fn try_take(&self) -> Option<T> {
        let mut lock = self.data.lock().unwrap();
        lock.take()
    }

    pub fn wait(&self) -> T {
        let mut lock = self.data.lock().unwrap();
        loop {
            match lock.take() {
                Some(res) => return res,
                None => lock = self.condition.wait(lock).unwrap(),
            }
        }
    }

    pub fn set(&self, res: T) {
        let mut lock = self.data.lock().unwrap();
        lock.replace(res);
        self.condition.notify_one();
    }
}

#[allow(dead_code)]
impl<T: Clone> Promise<T> {
    pub fn try_clone(&self) -> Option<T> {
        let lock = self.data.lock().unwrap();
        lock.as_ref().cloned()
    }

    pub fn wait_clone(&self) -> T {
        let mut lock = self.data.lock().unwrap();
        loop {
            match lock.as_ref() {
                Some(res) => return res.clone(),
                None => lock = self.condition.wait(lock).unwrap(),
            }
        }
    }
}

pub enum PromiseResult<T> {
    Ok { value: T },
    Cancelled,
}

#[derive(Debug, TryFromValue, PartialEq, Eq)]
#[irondash(tag = "type", rename_all = "camelCase")]
pub enum ValuePromiseResult {
    Ok { value: Value },
    Cancelled,
}

pub type ValuePromise = Promise<ValuePromiseResult>;

pub trait ValuePromiseSetCancel<V> {
    fn set_value(&self, v: V);
    fn cancel(&self);
}

impl ValuePromiseSetCancel<Value> for ValuePromise {
    fn set_value(&self, v: Value) {
        self.set(ValuePromiseResult::Ok { value: v });
    }

    fn cancel(&self) {
        self.set(ValuePromiseResult::Cancelled);
    }
}
