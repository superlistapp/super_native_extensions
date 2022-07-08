use std::{rc::Rc, slice, sync::Arc, thread};

use nativeshell_core::RunLoop;
use windows::{
    core::{implement, Interface},
    Win32::{
        Foundation::{E_FAIL, E_NOTIMPL, S_FALSE, S_OK},
        System::Com::{
            CoInitialize, CoUninitialize, ISequentialStream_Impl, IStream, IStream_Impl,
            Marshal::CoMarshalInterThreadInterfaceInStream,
            StructuredStorage::CoGetInterfaceAndReleaseStream, STREAM_SEEK,
        },
    },
};

use crate::{segmented_queue::SegmentedQueueReader, util::DropNotifier, value_promise::Promise};

#[implement(IStream)]
pub struct VirtualFileStream {
    run_loop: Rc<RunLoop>,
    reader: SegmentedQueueReader,
    size_promise: Arc<Promise<Option<i64>>>,
    error_promise: Arc<Promise<String>>,
    _drop_notifier: Arc<DropNotifier>,
}

impl VirtualFileStream {
    /// Moves the stream to another thread and return marshalled proxy
    pub fn create_on_another_thread(
        reader: SegmentedQueueReader,
        size_promise: Arc<Promise<Option<i64>>>,
        error_promise: Arc<Promise<String>>,
        drop_notifier: Arc<DropNotifier>,
    ) -> IStream {
        struct Movable<T>(T);
        unsafe impl<T> Send for Movable<T> {}
        let promise = Arc::new(Promise::<Movable<IStream>>::new());
        let promise_clone = promise.clone();
        thread::spawn(move || unsafe {
            CoInitialize(std::ptr::null_mut()).ok();
            let run_loop = Rc::new(RunLoop::new());
            let stream: IStream = VirtualFileStream {
                run_loop: run_loop.clone(),
                reader,
                size_promise,
                error_promise,
                _drop_notifier: drop_notifier,
            }
            .into();
            let mashalled = CoMarshalInterThreadInterfaceInStream(&IStream::IID, stream).unwrap();
            promise_clone.set(Movable(mashalled));
            run_loop.run(); // will be stopped in drop
            CoUninitialize();
        });

        let stream = promise.wait().0;
        let stream: IStream = unsafe { CoGetInterfaceAndReleaseStream(stream) }.unwrap();
        stream
    }
}

impl Drop for VirtualFileStream {
    fn drop(&mut self) {
        println!("Stream bye bye");
        self.run_loop.stop();
    }
}

impl ISequentialStream_Impl for VirtualFileStream {
    fn Read(
        &self,
        pv: *mut core::ffi::c_void,
        cb: u32,
        pcbread: *mut u32,
    ) -> windows::core::HRESULT {
        // This doesn't entirely conform to ISequentialStream::Read documentation.
        // To avoid blocking we may read less data than requested and still
        // return S_OK.
        let pcbread = unsafe { &mut *pcbread };
        let data = self.reader.read_some(cb as usize);
        let data_out = unsafe { slice::from_raw_parts_mut(pv as *mut u8, data.len()) };
        data_out.copy_from_slice(&data);
        *pcbread = data.len() as u32;

        if let Some(_err) = self.error_promise.try_clone() {
            // TODO(knopp): Can we somehow pass the message?
            return E_FAIL;
        }
        if data.len() > 0 as usize {
            S_OK
        } else {
            S_FALSE
        }
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
        println!(
            "SEEK {},{:?} {:?}",
            dlibmove,
            dworigin,
            thread::current().id()
        );
        Err(E_NOTIMPL.into())
        // Ok(0)
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
        _grfstatflag: u32,
    ) -> windows::core::Result<()> {
        let size = self.size_promise.wait_clone();
        let statstg = unsafe { &mut *pstatstg };
        statstg.cbSize = size.unwrap_or(0) as u64;
        println!("STAT {:?}", size);
        // Err(E_NOTIMPL.into())
        Ok(())
    }

    fn Clone(&self) -> windows::core::Result<IStream> {
        Err(E_NOTIMPL.into())
    }
}
