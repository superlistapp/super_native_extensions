use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    slice,
    sync::Arc,
    thread,
};

use nativeshell_core::{util::Capsule, RunLoop};
use windows::{
    core::{implement, Interface},
    Win32::{
        Foundation::{E_FAIL, E_NOTIMPL, S_FALSE, S_OK},
        System::Com::{
            CoInitialize, CoUninitialize, ISequentialStream_Impl, IStream, IStream_Impl,
            Marshal::CoMarshalInterThreadInterfaceInStream,
            StructuredStorage::CoGetInterfaceAndReleaseStream, STREAM_SEEK, STREAM_SEEK_CUR,
            STREAM_SEEK_SET,
        },
    },
};

use crate::{segmented_queue::SegmentedQueueReader, util::DropNotifier, value_promise::Promise};

struct StreamInner {
    run_loop: Rc<RunLoop>,
    reader: SegmentedQueueReader,
    size_promise: Arc<Promise<Option<i64>>>,
    error_promise: Arc<Promise<String>>,
    _drop_notifier: Arc<DropNotifier>,
    position: Cell<i64>,
}

struct Stream {
    inner: RefCell<Option<StreamInner>>,
}

impl Stream {
    fn dispose(&self) {
        self.inner.take();
    }

    fn read(
        &self,
        pv: *mut core::ffi::c_void,
        cb: u32,
        pcbread: *mut u32,
    ) -> windows::core::HRESULT {
        let inner = self.inner.borrow();
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
                if data.len() > 0 as usize {
                    S_OK
                } else {
                    S_FALSE
                }
            }
            None => S_FALSE,
        }
    }

    fn seek(&self, dlibmove: i64, dworigin: STREAM_SEEK) -> windows::core::Result<u64> {
        let inner = self.inner.borrow();
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
        _grfstatflag: u32,
    ) -> windows::core::Result<()> {
        let inner = self.inner.borrow();
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
        self.run_loop.stop();
    }
}

#[implement(IStream)]
pub struct VirtualFileStream {
    stream: Rc<Stream>,
}

impl VirtualFileStream {
    /// Moves the stream to another thread and return marshalled proxy
    pub fn create_on_another_thread(
        reader: SegmentedQueueReader,
        size_promise: Arc<Promise<Option<i64>>>,
        error_promise: Arc<Promise<String>>,
        drop_notifier: Arc<DropNotifier>, // fired when stream is dropped
    ) -> (IStream, Arc<DropNotifier>) {
        struct Movable<T>(T);
        unsafe impl<T> Send for Movable<T> {}
        let promise = Arc::new(Promise::<(Movable<IStream>, Box<dyn FnOnce() + Send>)>::new());
        let promise_clone = promise.clone();
        thread::spawn(move || unsafe {
            CoInitialize(std::ptr::null_mut()).ok();
            let run_loop = Rc::new(RunLoop::new());
            let stream = Rc::new(Stream {
                inner: RefCell::new(Some(StreamInner {
                    run_loop: run_loop.clone(),
                    reader,
                    size_promise,
                    error_promise,
                    _drop_notifier: drop_notifier,
                    position: Cell::new(0),
                })),
            });
            let stream_capsule = Capsule::new(stream.clone());
            let stream: IStream = VirtualFileStream { stream }.into();
            let mashalled = CoMarshalInterThreadInterfaceInStream(&IStream::IID, stream).unwrap();
            let sender = run_loop.new_sender();
            // Ensure stream disposal when parent dataobject is disposed. This is
            // to ensure that when stream leaks it is at least destroyed (and thread
            // released)  when data object is destroyed.
            // https://github.com/microsoft/terminal/issues/13498
            let clean_up = Box::new(move || {
                sender.send(move || {
                    stream_capsule.get_ref().unwrap().dispose();
                });
            });
            promise_clone.set((Movable(mashalled), clean_up));
            run_loop.run(); // will be stopped in drop
            CoUninitialize();
        });

        let res = promise.wait();
        let stream: IStream = unsafe { CoGetInterfaceAndReleaseStream(res.0 .0) }.unwrap();
        let drop_notifier = DropNotifier::new_with_boxed(res.1);
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

    fn LockRegion(&self, _liboffset: u64, _cb: u64, _dwlocktype: u32) -> windows::core::Result<()> {
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
        grfstatflag: u32,
    ) -> windows::core::Result<()> {
        self.stream.stat(pstatstg, grfstatflag)
    }

    fn Clone(&self) -> windows::core::Result<IStream> {
        Err(E_NOTIMPL.into())
    }
}
