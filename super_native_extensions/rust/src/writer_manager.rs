use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    sync::Arc,
};

use async_trait::async_trait;
use nativeshell_core::{
    util::Late, AsyncMethodHandler, AsyncMethodInvoker, Context, IntoPlatformResult, IsolateId,
    MethodCall, PlatformError, PlatformResult, RegisteredAsyncMethodHandler, Value,
};

use crate::{
    api_model::{ClipboardWriterData, LazyValueId, PlatformWriterId},
    error::{ClipboardError, ClipboardResult},
    platform::PlatformClipboardWriter,
    value_promise::{ValuePromise, ValuePromiseResult},
};

pub struct ClipboardWriterManager {
    weak_self: Late<Weak<Self>>,
    invoker: Late<AsyncMethodInvoker>,
    next_id: Cell<i64>,
    writers: RefCell<HashMap<PlatformWriterId, WriterEntry>>,
}

struct WriterEntry {
    isolate_id: IsolateId,
    platform_writer: Rc<PlatformClipboardWriter>,
}

pub trait GetClipboardWriterManager {
    fn clipboard_writer_manager(&self) -> Rc<ClipboardWriterManager>;
}

impl GetClipboardWriterManager for Context {
    fn clipboard_writer_manager(&self) -> Rc<ClipboardWriterManager> {
        self.get_attachment(ClipboardWriterManager::new).handler()
    }
}

impl ClipboardWriterManager {
    pub fn new() -> RegisteredAsyncMethodHandler<Self> {
        Self {
            weak_self: Late::new(),
            invoker: Late::new(),
            next_id: Cell::new(1),
            writers: RefCell::new(HashMap::new()),
        }
        .register("ClipboardWriterManager")
    }

    fn register_writer(
        &self,
        data: ClipboardWriterData,
        isolate_id: IsolateId,
    ) -> ClipboardResult<i64> {
        let id = self.next_id.get();
        self.next_id.replace(id + 1);
        let platform_clipboard = Rc::new(PlatformClipboardWriter::new(
            self.weak_self.clone(),
            isolate_id,
            data,
        ));
        platform_clipboard.assign_weak_self(Rc::downgrade(&platform_clipboard));
        self.writers.borrow_mut().insert(
            id.into(),
            WriterEntry {
                isolate_id,
                platform_writer: platform_clipboard,
            },
        );
        Ok(id)
    }

    fn unregister_writer(&self, writer: PlatformWriterId) -> ClipboardResult<()> {
        self.writers.borrow_mut().remove(&writer);
        Ok(())
    }

    pub fn get_platform_writer(
        &self,
        clipboard: PlatformWriterId,
    ) -> ClipboardResult<Rc<PlatformClipboardWriter>> {
        self.writers
            .borrow()
            .get(&clipboard)
            .map(|e| e.platform_writer.clone())
            .ok_or_else(|| ClipboardError::OtherError("Clipboard not found".into()))
    }

    async fn write_to_clipboard(&self, writer: PlatformWriterId) -> ClipboardResult<()> {
        self.get_platform_writer(writer)?.write_to_clipboard().await
    }
}

#[async_trait(?Send)]
impl AsyncMethodHandler for ClipboardWriterManager {
    async fn on_method_call(&self, call: MethodCall) -> PlatformResult {
        match call.method.as_str() {
            "registerClipboardWriter" => self
                .register_writer(call.args.try_into()?, call.isolate)
                .into_platform_result(),
            "unregisterClipboardWriter" => self
                .unregister_writer(call.args.try_into()?)
                .into_platform_result(),
            "writeToClipboard" => self
                .write_to_clipboard(call.args.try_into()?)
                .await
                .into_platform_result(),
            _ => Err(PlatformError {
                code: "invalid_method".into(),
                message: Some(format!("Unknown Method: {}", call.method)),
                detail: Value::Null,
            }),
        }
    }

    fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    fn assign_invoker(&self, invoker: AsyncMethodInvoker) {
        self.invoker.set(invoker);
    }

    // Called when engine is about to be destroyed.
    fn on_isolate_destroyed(&self, isolate_id: IsolateId) {
        let mut writers = self.writers.borrow_mut();
        let writers_to_remove: Vec<_> = writers
            .iter()
            .filter_map(|(id, writer)| {
                if writer.isolate_id == isolate_id {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        for writer_id in writers_to_remove {
            writers.remove(&writer_id);
        }
    }
}

#[async_trait(?Send)]
pub trait PlatformClipboardWriterDelegate {
    fn get_lazy_data(
        &self,
        isolate_id: IsolateId,
        data_id: LazyValueId,
        on_done: Option<Box<dyn FnOnce()>>,
    ) -> Arc<ValuePromise>;

    async fn get_lazy_data_async(
        &self,
        isolate_id: IsolateId,
        data_id: LazyValueId,
    ) -> ValuePromiseResult;
}

#[async_trait(?Send)]
impl PlatformClipboardWriterDelegate for ClipboardWriterManager {
    fn get_lazy_data(
        &self,
        isolate_id: IsolateId,
        data_id: LazyValueId,
        on_done: Option<Box<dyn FnOnce()>>,
    ) -> Arc<ValuePromise> {
        let res = Arc::new(ValuePromise::new());
        let res_clone = res.clone();
        let weak_self = self.weak_self.clone();
        Context::get().run_loop().spawn(async move {
            let this = weak_self.upgrade();
            if let Some(this) = this {
                let res = this.get_lazy_data_async(isolate_id, data_id).await;
                res_clone.set(res);
                if let Some(on_done) = on_done {
                    on_done();
                }
            } else {
                res_clone.cancel();
            }
        });
        res
    }

    async fn get_lazy_data_async(
        &self,
        isolate_id: IsolateId,
        data_id: LazyValueId,
    ) -> ValuePromiseResult {
        let res = self
            .invoker
            .call_method_cv(isolate_id, "getLazyData", data_id)
            .await;
        match res {
            Ok(res) => res,
            Err(_) => ValuePromiseResult::Cancelled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ClipboardWriterManager;
    use crate::{
        platform::WRITERS,
        writer_data::{ClipboardWriterData, ClipboardWriterItem, ClipboardWriterItemData},
    };
    use nativeshell_core::{Context, GetMessageChannel, MockIsolate};
    use velcro::hash_map;

    async fn test1_main() {
        let _writer_manager = ClipboardWriterManager::new();
        let context = Context::get();
        let channel = "ClipboardWriterManager";

        let isolate_1 = MockIsolate::new();
        isolate_1.register_method_handler(channel, |call, reply| {
            if call.method == "getLazyData" {
                let id: i64 = call.args.try_into().unwrap();
                if id == 10 {
                    reply(Ok(vec![
                        ("type".into(), "ok".into()),
                        ("value".into(), "SomeValue".into()),
                    ]
                    .into()));
                } else {
                    reply(Ok(vec![("type".into(), "cancelled".into())].into()));
                }
            } else {
                panic!("Unexpected method call {:?}", call);
            }
        });
        let isolate_1 = isolate_1.attach(&context.message_channel());

        let isolate_2 = MockIsolate::new();
        isolate_2.register_method_handler(channel, |call, reply| {
            if call.method == "getLazyData" {
                let id: i64 = call.args.try_into().unwrap();
                if id == 20 {
                    reply(Ok(vec![
                        ("type".into(), "ok".into()),
                        ("value".into(), "AnotherValue".into()),
                    ]
                    .into()));
                } else {
                    reply(Ok(vec![("type".into(), "cancelled".into())].into()));
                }
            } else {
                panic!("Unexpected method call {:?}", call);
            }
        });
        let isolate_2 = isolate_2.attach(&context.message_channel());

        let data_1 = ClipboardWriterData {
            items: vec![
                ClipboardWriterItem {
                    data: vec![ClipboardWriterItemData::Simple {
                        types: vec!["type1".to_owned()],
                        data: "value".into(),
                    }],
                },
                ClipboardWriterItem {
                    data: vec![
                        ClipboardWriterItemData::Lazy {
                            types: vec!["type2".to_owned()],
                            id: 10,
                        },
                        ClipboardWriterItemData::Lazy {
                            types: vec!["type3".to_owned()],
                            id: 11,
                        },
                    ],
                },
            ],
        };

        let data_2 = ClipboardWriterData {
            items: vec![
                ClipboardWriterItem {
                    data: vec![ClipboardWriterItemData::Simple {
                        types: vec!["type1a".to_owned()],
                        data: "value2".into(),
                    }],
                },
                ClipboardWriterItem {
                    data: vec![ClipboardWriterItemData::Lazy {
                        types: vec!["type2a".to_owned()],
                        id: 20,
                    }],
                },
            ],
        };

        assert_eq!(WRITERS.with(|c| c.borrow().len()), 0);

        let res_1 = isolate_1
            .call_method_async(channel, "registerClipboardWriter", data_1.clone().into())
            .await
            .unwrap();

        let res_1: i64 = res_1.try_into().unwrap();
        assert_eq!(res_1, 1);

        assert_eq!(WRITERS.with(|c| c.borrow().len()), 1);

        let writer_1 = WRITERS.with(|a| a.borrow().last().unwrap().clone());
        {
            let writer_1 = writer_1.upgrade().unwrap();
            assert_eq!(isolate_1.isolate_id(), writer_1.isolate_id);
            assert!(writer_1.written_data.borrow().is_none());
        }

        let res_2 = isolate_2
            .call_method_async(channel, "registerClipboardWriter", data_2.clone().into())
            .await
            .unwrap();

        let res_2: i64 = res_2.try_into().unwrap();
        assert_eq!(res_2, 2);

        assert_eq!(WRITERS.with(|c| c.borrow().len()), 2);

        let writer_2 = WRITERS.with(|a| a.borrow().last().unwrap().clone());
        {
            let writer_2 = writer_1.upgrade().unwrap();
            assert_eq!(isolate_1.isolate_id(), writer_2.isolate_id);
            assert!(writer_2.written_data.borrow().is_none());
        }

        isolate_1
            .call_method_async(channel, "writeToClipboard", res_1.into())
            .await
            .unwrap();

        {
            let writer_1 = writer_1.upgrade().unwrap();
            assert_eq!(writer_1.written_data.borrow().as_ref().unwrap(), &data_1);
        }

        isolate_2
            .call_method_async(channel, "writeToClipboard", res_2.into())
            .await
            .unwrap();

        {
            let writer_2 = writer_2.upgrade().unwrap();
            assert_eq!(writer_2.written_data.borrow().as_ref().unwrap(), &data_2);
        }

        {
            let writer_1 = writer_1.upgrade().unwrap();
            writer_1.request_all_lazy_items().await;
            assert_eq!(
                *writer_1.lazy_data.borrow(),
                hash_map! {
                    10: crate::value_promise::ValuePromiseResult::Ok { value: "SomeValue".into() },
                    11: crate::value_promise::ValuePromiseResult::Cancelled,
                }
            );
        }

        {
            let writer_2 = writer_2.upgrade().unwrap();
            writer_2.request_lazy_item(20).await;
            writer_2.request_lazy_item(21).await;
            assert_eq!(
                *writer_2.lazy_data.borrow(),
                hash_map! {
                    20: crate::value_promise::ValuePromiseResult::Ok { value: "AnotherValue".into() },
                    21: crate::value_promise::ValuePromiseResult::Cancelled,
                }
            );
        }

        isolate_1
            .call_method_async(channel, "unregisterClipboardWriter", res_1.into())
            .await
            .unwrap();

        assert_eq!(WRITERS.with(|c| c.borrow().len()), 1);

        // Simulate isolate shut down
        drop(isolate_2);

        assert_eq!(WRITERS.with(|c| c.borrow().len()), 0);
    }

    #[test]
    fn test1() {
        Context::run_test(test1_main());
    }
}
