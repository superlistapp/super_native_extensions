use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    mem::{size_of, ManuallyDrop},
    process::id,
    rc::{Rc, Weak},
    slice,
    sync::{Arc, Condvar, Mutex},
    thread::{self, Thread},
    time::Duration,
};

use nativeshell_core::{util::Late, IsolateId, RunLoop};
use windows::{
    core::{implement, Interface, HRESULT},
    Win32::{
        Foundation::{
            BOOL, DATA_S_SAMEFORMATETC, DV_E_FORMATETC, E_NOTIMPL, E_OUTOFMEMORY, HWND,
            OLE_E_ADVISENOTSUPPORTED, POINT, S_FALSE, S_OK,
        },
        System::{
            Com::{
                CoInitialize, CoUninitialize, IBindCtx, IDataObject, IDataObject_Impl,
                ISequentialStream_Impl, IStream, IStream_Impl,
                Marshal::CoMarshalInterThreadInterfaceInStream,
                StructuredStorage::CoGetInterfaceAndReleaseStream, DATADIR_GET, FORMATETC,
                STGMEDIUM, STGMEDIUM_0, STREAM_SEEK, STREAM_SEEK_END, TYMED_HGLOBAL, TYMED_ISTREAM,
            },
            DataExchange::RegisterClipboardFormatW,
            Memory::{
                GlobalAlloc, GlobalFree, GlobalLock, GlobalSize, GlobalUnlock, GLOBAL_ALLOC_FLAGS,
            },
            Ole::{OleSetClipboard, ReleaseStgMedium},
            SystemServices::CF_HDROP,
        },
        UI::{
            Shell::{
                IDataObjectAsyncCapability, IDataObjectAsyncCapability_Impl, SHCreateMemStream,
                SHCreateStdEnumFmtEtc, CFSTR_DROPDESCRIPTION, CFSTR_FILECONTENTS,
                CFSTR_FILEDESCRIPTOR, DROPFILES, FD_ATTRIBUTES, FD_PROGRESSUI, FILEDESCRIPTORA,
                FILEDESCRIPTORW, FILEGROUPDESCRIPTORW,
            },
            WindowsAndMessaging::{
                DispatchMessageW, FindWindowExW, MsgWaitForMultipleObjects, PeekMessageW,
                TranslateMessage, HWND_MESSAGE, MSG, PM_NOYIELD, PM_REMOVE, QS_POSTMESSAGE,
            },
        },
    },
};

use crate::{
    api_model::{DataSource, DataSourceItemRepresentation, DataSourceValueId},
    data_source_manager::PlatformDataSourceDelegate,
    error::NativeExtensionsResult,
    util::DropNotifier,
    value_coerce::{CoerceToData, StringFormat},
    value_promise::{Promise, ValuePromise, ValuePromiseResult},
};

use super::{
    common::{
        as_u8_slice, format_from_string, format_to_string, make_format_with_tymed,
        make_format_with_tymed_index, message_loop_hwnds, pump_message_loop,
    },
    data_object::DataObject,
};

pub fn platform_stream_write(handle: i32, data: &[u8]) -> i32 {
    todo!()
}

pub fn platform_stream_close(handle: i32, delete: bool) {
    todo!()
}

pub struct PlatformDataSource {
    weak_self: Late<Weak<Self>>,
    pub(super) isolate_id: IsolateId,
    pub(super) delegate: Weak<dyn PlatformDataSourceDelegate>,
    pub(super) data: DataSource,
}

impl PlatformDataSource {
    pub fn new(
        delegate: Weak<dyn PlatformDataSourceDelegate>,
        isolate_id: IsolateId,
        data: DataSource,
    ) -> Self {
        Self {
            weak_self: Late::new(),
            isolate_id,
            delegate,
            data,
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    pub async fn write_to_clipboard(
        &self,
        drop_notifier: Arc<DropNotifier>,
    ) -> NativeExtensionsResult<()> {
        let data_object = DataObject::create(self.weak_self.upgrade().unwrap(), drop_notifier);
        unsafe {
            OleSetClipboard(data_object)?;
        }
        Ok(())
    }
}

#[no_mangle]
unsafe extern "C" fn break_here() {
    // println!("BREAK HERE");
}

#[implement(IStream)]
pub struct MyStream {
    run_loop: Rc<RunLoop>,
    num_read: Cell<i64>,
}

impl MyStream {
    /// Moves the stream to another thread and return marshalled proxy
    pub fn create_on_another_thread() -> IStream {
        struct Movable<T>(T);
        unsafe impl<T> Send for Movable<T> {}
        let promise = Arc::new(Promise::<Movable<IStream>>::new());
        let promise_clone = promise.clone();
        thread::spawn(move || unsafe {
            CoInitialize(std::ptr::null_mut()).ok();
            let run_loop = Rc::new(RunLoop::new());
            let stream: IStream = MyStream {
                run_loop: run_loop.clone(),
                num_read: Cell::new(0),
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

impl Drop for MyStream {
    fn drop(&mut self) {
        println!("Stream bye bye");
        self.run_loop.stop();
    }
}

impl ISequentialStream_Impl for MyStream {
    fn Read(
        &self,
        pv: *mut core::ffi::c_void,
        cb: u32,
        pcbread: *mut u32,
    ) -> windows::core::HRESULT {
        unsafe { break_here() };
        // thread::sleep(Duration::from_secs(5));
        // println!("READ  {:?} {:?}", cb, thread::current().id());
        let pcbread = unsafe { &mut *pcbread };
        if self.num_read.get() > 1024 {
            *pcbread = 0;
        } else {
            *pcbread = 10;
        }
        self.num_read.replace(self.num_read.get() + *pcbread as i64);
        thread::sleep(Duration::from_millis(100));
        S_OK
        // E_NOTIMPL
    }

    fn Write(
        &self,
        pv: *const core::ffi::c_void,
        cb: u32,
        pcbwritten: *mut u32,
    ) -> windows::core::HRESULT {
        E_NOTIMPL
    }
}

impl IStream_Impl for MyStream {
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
        grfcommitflags: windows::Win32::System::Com::STGC,
    ) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn Revert(&self) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn LockRegion(&self, liboffset: u64, cb: u64, dwlocktype: u32) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn UnlockRegion(&self, liboffset: u64, cb: u64, dwlocktype: u32) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn Stat(
        &self,
        pstatstg: *mut windows::Win32::System::Com::STATSTG,
        grfstatflag: u32,
    ) -> windows::core::Result<()> {
        let statstg = unsafe { &mut *pstatstg };
        statstg.cbSize = 1024;
        // println!("STAT");
        // Err(E_NOTIMPL.into())
        Ok(())
    }

    fn Clone(&self) -> windows::core::Result<IStream> {
        Err(E_NOTIMPL.into())
    }
}
