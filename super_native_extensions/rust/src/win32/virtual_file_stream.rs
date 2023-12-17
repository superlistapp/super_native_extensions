use std::{
    cell::Cell,
    slice,
    sync::{Arc, Mutex},
};

use irondash_run_loop::{util::Capsule, RunLoop, RunLoopSender};
use threadpool::ThreadPool;
use windows::{
    core::{implement, ComInterface},
    Win32::{
        Foundation::{E_FAIL, E_NOTIMPL, S_FALSE, S_OK},
        System::Com::{
            CoInitialize, CoUninitialize, ISequentialStream_Impl, IStream, IStream_Impl,
            Marshal::CoMarshalInterThreadInterfaceInStream,
            StructuredStorage::CoGetInterfaceAndReleaseStream, LOCKTYPE, STATFLAG, STREAM_SEEK,
            STREAM_SEEK_CUR, STREAM_SEEK_SET,
        },
    },
};

use crate::{
    data_provider_manager::VirtualSessionHandle,
    segmented_queue::SegmentedQueueReader,
    util::{DropNotifier, Movable},
    value_promise::Promise,
};

struct StreamState {
    reader: SegmentedQueueReader,
    size_promise: Arc<Promise<Option<i64>>>,
    error_promise: Arc<Promise<String>>,
    _handle: Arc<VirtualSessionHandle>,
    position: Cell<i64>,
}

struct StreamInner {
    stream: Option<StreamState>,
    factory: Option<Box<dyn FnOnce() -> StreamState + Send>>,
    run_loop_sender: Option<RunLoopSender>,
}

struct Stream {
    inner: Mutex<StreamInner>,
}

impl Stream {
    fn initialize_if_needed(&self) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(factory) = inner.factory.take() {
            inner.stream.replace(factory());
        }
    }

    fn dispose_inactive_stream(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.factory.take();

        // If client didn't interact with the stream until now shut down the
        // RunLoop thread.
        if inner.stream.is_none() {
            if let Some(sender) = inner.run_loop_sender.take() {
                sender.send(move || {
                    RunLoop::current().stop();
                });
            }
        }
    }

    fn read(
        &self,
        pv: *mut core::ffi::c_void,
        cb: u32,
        pcbread: *mut u32,
    ) -> windows::core::HRESULT {
        self.initialize_if_needed();
        let inner = self.inner.lock().unwrap();
        match inner.stream.as_ref() {
            Some(stream) => {
                // This doesn't entirely conform to ISequentialStream::Read documentation.
                // To avoid blocking we may read less data than requested and still
                // return S_OK.
                let pcbread = unsafe { &mut *pcbread };
                let data = stream.reader.read_some(cb as usize);
                let data_out = unsafe { slice::from_raw_parts_mut(pv as *mut u8, data.len()) };
                data_out.copy_from_slice(&data);
                *pcbread = data.len() as u32;

                stream
                    .position
                    .set(stream.position.get() + data.len() as i64);

                if let Some(_err) = stream.error_promise.try_clone() {
                    // TODO(knopp): Can we somehow pass the message?
                    return E_FAIL;
                }
                if !data.is_empty() {
                    S_OK
                } else {
                    S_FALSE
                }
            }
            None => S_FALSE,
        }
    }

    fn seek(&self, dlibmove: i64, dworigin: STREAM_SEEK) -> windows::core::Result<u64> {
        self.initialize_if_needed();
        let inner = self.inner.lock().unwrap();
        match inner.stream.as_ref() {
            Some(stream) => {
                let position = if dworigin == STREAM_SEEK_SET {
                    dlibmove
                } else if dworigin == STREAM_SEEK_CUR {
                    stream.position.get() + dlibmove
                } else {
                    -1
                };
                // Pretend that seek is supported as long as we don't really need to seek
                if position == stream.position.get() {
                    Ok(position as u64)
                } else {
                    Err(E_NOTIMPL.into())
                }
            }
            None => Err(E_NOTIMPL.into()),
        }
    }

    fn stat(
        &self,
        pstatstg: *mut windows::Win32::System::Com::STATSTG,
        _grfstatflag: &STATFLAG,
    ) -> windows::core::Result<()> {
        self.initialize_if_needed();
        let size_promise = {
            let inner = self.inner.lock().unwrap();
            inner
                .stream
                .as_ref()
                .map(|inner| inner.size_promise.clone())
        };

        let size = size_promise.and_then(|p| p.wait_clone());
        let statstg = unsafe { &mut *pstatstg };
        statstg.cbSize = size.unwrap_or(0) as u64;

        Ok(())
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        if let Some(sender) = self.inner.lock().unwrap().run_loop_sender.take() {
            sender.send(move || {
                RunLoop::current().stop();
            });
        }
    }
}

pub struct VirtualStreamSession {
    pub reader: SegmentedQueueReader,
    pub size_promise: Arc<Promise<Option<i64>>>,
    pub error_promise: Arc<Promise<String>>,
    pub handle: Arc<VirtualSessionHandle>,
}

// Represents the session provider as boxed factory method that is Send
fn session_provider_as_factory<F>(provider: F) -> Box<dyn FnOnce() -> StreamState + 'static + Send>
where
    F: FnOnce() -> VirtualStreamSession + 'static,
{
    let sender = RunLoop::current().new_sender();
    let mut provider = Capsule::new_with_sender(provider, sender.clone());
    Box::new(move || {
        let res = Arc::new(Promise::new());
        let res_clone = res.clone();
        sender.send(move || {
            let provider = provider.take().unwrap();
            let session = provider();
            res_clone.set(StreamState {
                reader: session.reader,
                size_promise: session.size_promise,
                error_promise: session.error_promise,
                _handle: session.handle,
                position: Cell::new(0),
            })
        });
        res.wait()
    })
}

#[implement(IStream)]
pub struct VirtualFileStream {
    stream: Arc<Stream>,
}

impl VirtualFileStream {
    /// Creates agile stream that can be accessed on any thread. This should be
    /// used when stream is accessed by local application on background thread.
    pub fn create_agile<F>(session_provider: F) -> (IStream, Arc<DropNotifier>)
    where
        F: FnOnce() -> VirtualStreamSession + 'static,
    {
        let factory = session_provider_as_factory(session_provider);
        let stream = Arc::new(Stream {
            inner: Mutex::new(StreamInner {
                stream: None,
                factory: Some(factory),
                run_loop_sender: None,
            }),
        });
        let stream: IStream = VirtualFileStream { stream }.into();
        (
            stream,
            Arc::new(DropNotifier::new(move || {
                // We don't let the stream leak, no need for manual dispose here.
            })),
        )
    }

    /// Moves the stream to another thread and return marshalled proxy.
    /// This should be used when providing stream to other applications.
    pub fn create_marshalled_on_background_thread<F>(
        session_provider: F,
        thread_pool: &mut ThreadPool,
    ) -> (IStream, Arc<DropNotifier>)
    where
        F: FnOnce() -> VirtualStreamSession + 'static,
    {
        let factory = session_provider_as_factory(session_provider);
        let promise = Arc::new(Promise::<(Movable<IStream>, Box<dyn FnOnce() + Send>)>::new());
        let promise_clone = promise.clone();

        // VSCode seems to periodically request the stream just to close it immediately.
        // To alleviate this a bit we try to reuse the thread. However we also want to
        // ensure that we don't wait on thread_pool because existing running tasks might be
        // blocking.
        if thread_pool.active_count() == thread_pool.max_count() {
            thread_pool.set_num_threads(thread_pool.max_count() + 1);
        }
        thread_pool.execute(move || unsafe {
            CoInitialize(None).ok();
            {
                let stream = Arc::new(Stream {
                    inner: Mutex::new(StreamInner {
                        stream: None,
                        factory: Some(factory),
                        run_loop_sender: Some(RunLoop::current().new_sender()),
                    }),
                });
                let weak_stream = Arc::downgrade(&stream);
                let stream: IStream = VirtualFileStream { stream }.into();
                let mashalled =
                    CoMarshalInterThreadInterfaceInStream(&IStream::IID, &stream).unwrap();
                // Ensure stream disposal when parent dataobject is disposed. This is
                // to ensure that when stream leaks it is at least destroyed (and thread
                // released)  when data object is destroyed.
                // https://github.com/microsoft/terminal/issues/13498
                let clean_up = Box::new(move || {
                    if let Some(stream) = weak_stream.upgrade() {
                        stream.dispose_inactive_stream();
                    }
                });
                promise_clone.set((Movable::new(mashalled), clean_up));
            }
            RunLoop::current().run(); // will be stopped in drop
            CoUninitialize();
        });

        let res = promise.wait();
        let stream = res.0.take();
        let stream: IStream = unsafe { CoGetInterfaceAndReleaseStream(&stream) }.unwrap();
        let drop_notifier = Arc::new(DropNotifier::new_with_boxed(res.1));
        (stream, drop_notifier)
    }
}

#[allow(non_snake_case)]
impl ISequentialStream_Impl for VirtualFileStream {
    fn Read(
        &self,
        pv: *mut core::ffi::c_void,
        cb: u32,
        pcbread: *mut u32,
    ) -> windows::core::HRESULT {
        self.stream.read(pv, cb, pcbread)
    }

    fn Write(
        &self,
        _pv: *const core::ffi::c_void,
        _cb: u32,
        _pcbwritten: *mut u32,
    ) -> windows::core::HRESULT {
        E_NOTIMPL
    }
}

#[allow(non_snake_case)]
impl IStream_Impl for VirtualFileStream {
    fn Seek(
        &self,
        dlibmove: i64,
        dworigin: STREAM_SEEK,
        plibnewposition: *mut u64,
    ) -> windows::core::Result<()> {
        let position = self.stream.seek(dlibmove, dworigin)?;
        if !plibnewposition.is_null() {
            let new_position = &mut unsafe { *plibnewposition };
            *new_position = position;
        }
        Ok(())
    }

    fn SetSize(&self, _libnewsize: u64) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn CopyTo(
        &self,
        _pstm: Option<&IStream>,
        _cb: u64,
        _pcbread: *mut u64,
        _pcbwritten: *mut u64,
    ) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn Commit(
        &self,
        _grfcommitflags: &windows::Win32::System::Com::STGC,
    ) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn Revert(&self) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn LockRegion(
        &self,
        _liboffset: u64,
        _cb: u64,
        _dwlocktype: &LOCKTYPE,
    ) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn UnlockRegion(
        &self,
        _liboffset: u64,
        _cb: u64,
        _dwlocktype: u32,
    ) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn Stat(
        &self,
        pstatstg: *mut windows::Win32::System::Com::STATSTG,
        grfstatflag: &STATFLAG,
    ) -> windows::core::Result<()> {
        self.stream.stat(pstatstg, grfstatflag)
    }

    fn Clone(&self) -> windows::core::Result<IStream> {
        Err(E_NOTIMPL.into())
    }
}
