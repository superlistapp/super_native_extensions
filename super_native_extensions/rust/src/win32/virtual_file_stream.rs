use std::{
    cell::Cell,
    slice,
    sync::{Arc, Mutex},
    thread,
};

use irondash_run_loop::{RunLoop, RunLoopSender};
use windows::{
    core::{implement, Interface},
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

struct StreamInner {
    run_loop_sender: Option<RunLoopSender>,
    reader: SegmentedQueueReader,
    size_promise: Arc<Promise<Option<i64>>>,
    error_promise: Arc<Promise<String>>,
    _handle: Arc<VirtualSessionHandle>,
    position: Cell<i64>,
}

struct Stream {
    inner: Mutex<Option<StreamInner>>,
}

impl Stream {
    fn dispose(&self) {
        self.inner.lock().unwrap().take();
    }

    fn read(
        &self,
        pv: *mut core::ffi::c_void,
        cb: u32,
        pcbread: *mut u32,
    ) -> windows::core::HRESULT {
        let inner = self.inner.lock().unwrap();
        match inner.as_ref() {
            Some(inner) => {
                // This doesn't entirely conform to ISequentialStream::Read documentation.
                // To avoid blocking we may read less data than requested and still
                // return S_OK.
                let pcbread = unsafe { &mut *pcbread };
                let data = inner.reader.read_some(cb as usize);
                let data_out = unsafe { slice::from_raw_parts_mut(pv as *mut u8, data.len()) };
                data_out.copy_from_slice(&data);
                *pcbread = data.len() as u32;

                inner.position.set(inner.position.get() + data.len() as i64);

                if let Some(_err) = inner.error_promise.try_clone() {
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
        let inner = self.inner.lock().unwrap();
        match inner.as_ref() {
            Some(inner) => {
                let position = if dworigin == STREAM_SEEK_SET {
                    dlibmove
                } else if dworigin == STREAM_SEEK_CUR {
                    inner.position.get() + dlibmove
                } else {
                    -1
                };
                // Pretend that seek is supported as long as we don't really need to seek
                if position == inner.position.get() {
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
        _grfstatflag: STATFLAG,
    ) -> windows::core::Result<()> {
        let inner = self.inner.lock().unwrap();
        if let Some(inner) = inner.as_ref() {
            let size = inner.size_promise.wait_clone();
            let statstg = unsafe { &mut *pstatstg };
            statstg.cbSize = size.unwrap_or(0) as u64;
        }
        Ok(())
    }
}

impl Drop for StreamInner {
    fn drop(&mut self) {
        if let Some(sender) = self.run_loop_sender.take() {
            sender.send(move || {
                RunLoop::current().stop();
            });
        }
    }
}

#[implement(IStream)]
pub struct VirtualFileStream {
    stream: Arc<Stream>,
}

impl VirtualFileStream {
    /// Creates agile stream that can be accessed on any thread. This should be
    /// used when stream is accessed by local applicaiton on background thread.
    pub fn create_agile(
        reader: SegmentedQueueReader,
        size_promise: Arc<Promise<Option<i64>>>,
        error_promise: Arc<Promise<String>>,
        handle: Arc<VirtualSessionHandle>,
    ) -> (IStream, Arc<DropNotifier>) {
        let stream = Arc::new(Stream {
            inner: Mutex::new(Some(StreamInner {
                run_loop_sender: None,
                reader,
                size_promise,
                error_promise,
                _handle: handle,
                position: Cell::new(0),
            })),
        });
        let stream_clone = stream.clone();
        let stream: IStream = VirtualFileStream { stream }.into();
        (
            stream,
            Arc::new(DropNotifier::new(move || {
                stream_clone.dispose();
            })),
        )
    }

    /// Moves the stream to another thread and return marshalled proxy.
    /// This should be used when providing stream to other applications.
    pub fn create_marshalled_on_background_thread(
        reader: SegmentedQueueReader,
        size_promise: Arc<Promise<Option<i64>>>,
        error_promise: Arc<Promise<String>>,
        handle: Arc<VirtualSessionHandle>, // fired when stream is dropped
    ) -> (IStream, Arc<DropNotifier>) {
        let promise = Arc::new(Promise::<(Movable<IStream>, Box<dyn FnOnce() + Send>)>::new());
        let promise_clone = promise.clone();
        thread::spawn(move || unsafe {
            CoInitialize(None).ok();
            let stream = Arc::new(Stream {
                inner: Mutex::new(Some(StreamInner {
                    run_loop_sender: Some(RunLoop::current().new_sender()),
                    reader,
                    size_promise,
                    error_promise,
                    _handle: handle,
                    position: Cell::new(0),
                })),
            });
            let stream_clone = stream.clone();
            let stream: IStream = VirtualFileStream { stream }.into();
            let mashalled = CoMarshalInterThreadInterfaceInStream(&IStream::IID, &stream).unwrap();
            // Ensure stream disposal when parent dataobject is disposed. This is
            // to ensure that when stream leaks it is at least destroyed (and thread
            // released)  when data object is destroyed.
            // https://github.com/microsoft/terminal/issues/13498
            let clean_up = Box::new(move || {
                stream_clone.dispose();
            });
            promise_clone.set((Movable::new(mashalled), clean_up));
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

impl IStream_Impl for VirtualFileStream {
    fn Seek(&self, dlibmove: i64, dworigin: STREAM_SEEK) -> windows::core::Result<u64> {
        self.stream.seek(dlibmove, dworigin)
    }

    fn SetSize(&self, _libnewsize: u64) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn CopyTo(
        &self,
        _pstm: &core::option::Option<IStream>,
        _cb: u64,
        _pcbread: *mut u64,
        _pcbwritten: *mut u64,
    ) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn Commit(
        &self,
        _grfcommitflags: windows::Win32::System::Com::STGC,
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
        _dwlocktype: LOCKTYPE,
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
        grfstatflag: STATFLAG,
    ) -> windows::core::Result<()> {
        self.stream.stat(pstatstg, grfstatflag)
    }

    fn Clone(&self) -> windows::core::Result<IStream> {
        Err(E_NOTIMPL.into())
    }
}
