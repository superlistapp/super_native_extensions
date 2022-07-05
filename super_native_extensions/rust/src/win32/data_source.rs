use std::{
    mem::{size_of, ManuallyDrop},
    rc::{Rc, Weak}, sync::Arc,
};

use nativeshell_core::{util::Late, IsolateId};
use windows::{
    core::implement,
    Win32::{
        Foundation::{
            DATA_S_SAMEFORMATETC, DV_E_FORMATETC, E_NOTIMPL, E_OUTOFMEMORY, HWND,
            OLE_E_ADVISENOTSUPPORTED, POINT, S_FALSE,
        },
        System::{
            Com::{
                IDataObject, IDataObject_Impl, IStream, DATADIR_GET, FORMATETC, STGMEDIUM,
                STGMEDIUM_0, STREAM_SEEK_END, TYMED_HGLOBAL, TYMED_ISTREAM,
            },
            Memory::{GlobalAlloc, GlobalFree, GlobalLock, GlobalUnlock, GLOBAL_ALLOC_FLAGS},
            Ole::OleSetClipboard,
            SystemServices::CF_HDROP,
        },
        UI::{
            Shell::{SHCreateMemStream, SHCreateStdEnumFmtEtc, DROPFILES},
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
    value_promise::ValuePromiseResult,
};

use super::common::{as_u8_slice, format_from_string, format_to_string, make_format_with_tymed};

pub fn platform_stream_write(handle: i32, data: &[u8]) -> i32 {
    todo!()
}

pub fn platform_stream_close(handle: i32, delete: bool) {
    todo!()
}

pub struct PlatformDataSource {
    weak_self: Late<Weak<Self>>,
    isolate_id: IsolateId,
    delegate: Weak<dyn PlatformDataSourceDelegate>,
    data: DataSource,
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
        let data_object = DataObject::create(self.weak_self.upgrade().unwrap());
        unsafe {
            OleSetClipboard(data_object)?;
        }
        Ok(())
    }
}

#[implement(IDataObject)]
struct DataObject {
    writer: Rc<PlatformDataSource>,
}

struct IStreamWrapper(IStream);
unsafe impl Send for IStreamWrapper {}

impl DataObject {
    fn create(writer: Rc<PlatformDataSource>) -> IDataObject {
        let data_object = Self { writer };
        data_object.into()
    }

    fn global_from_data(&self, data: &[u8]) -> windows::core::Result<isize> {
        unsafe {
            let global = GlobalAlloc(GLOBAL_ALLOC_FLAGS(0), data.len());
            let global_data = GlobalLock(global);
            if global_data.is_null() {
                GlobalFree(global);
                Err(E_OUTOFMEMORY.into())
            } else {
                std::ptr::copy_nonoverlapping(data.as_ptr(), global_data as *mut u8, data.len());
                GlobalUnlock(global);
                Ok(global)
            }
        }
    }

    fn lazy_data_for_id(&self, format: String, id: DataSourceValueId) -> Option<Vec<u8>> {
        let delegate = self.writer.delegate.upgrade();
        if let Some(delegate) = delegate {
            // Find hwnds of our task runner and flutter task runner
            let mut hwnds = Vec::<HWND>::new();
            unsafe {
                hwnds.push(FindWindowExW(
                    HWND_MESSAGE,
                    None,
                    "FlutterTaskRunnerWindow",
                    None,
                ));
                // There might be multiple nativeshell core event loops in the process, find
                // all hwnds
                let mut last: HWND = HWND(0);
                loop {
                    last = FindWindowExW(HWND_MESSAGE, last, "NativeShellCoreMessageWindow", None);
                    if last.0 != 0 {
                        hwnds.push(last);
                    } else {
                        break;
                    }
                }
            };
            let data = delegate.get_lazy_data(self.writer.isolate_id, id, format, None);
            loop {
                match data.try_take() {
                    Some(ValuePromiseResult::Ok { value }) => {
                        return value.coerce_to_data(StringFormat::Utf16NullTerminated)
                    }
                    Some(ValuePromiseResult::Cancelled) => return None,
                    None => unsafe {
                        // Process messages, but only from ours and flutter event loop
                        MsgWaitForMultipleObjects(&[], false, 10000000, QS_POSTMESSAGE);
                        let mut message = MSG::default();
                        loop {
                            let res = hwnds.iter().any(|hwnd| {
                                PeekMessageW(
                                    &mut message as *mut _,
                                    hwnd,
                                    0,
                                    0,
                                    PM_REMOVE | PM_NOYIELD,
                                )
                                .into()
                            });
                            if res {
                                TranslateMessage(&message as *const _);
                                DispatchMessageW(&message as *const _);
                            } else {
                                break;
                            }
                        }
                    },
                }
            }
        } else {
            None
        }
    }

    fn data_for_format(&self, format: u32, index: usize) -> Option<Vec<u8>> {
        let item = self.writer.data.items.get(index);
        if let Some(item) = item {
            let format = format_to_string(format);
            for representation in &item.representations {
                match representation {
                    DataSourceItemRepresentation::Simple { formats, data } => {
                        for ty in formats {
                            if ty == &format {
                                return data.coerce_to_data(StringFormat::Utf16NullTerminated);
                            }
                        }
                    }
                    DataSourceItemRepresentation::Lazy { formats, id } => {
                        for ty in formats {
                            if ty == &format {
                                return self.lazy_data_for_id(ty.clone(), *id);
                            }
                        }
                    }
                    _ => {}
                }
            }
            None
        } else {
            None
        }
    }

    // Bundles slice of utf16 encoded string into CF_HDROP
    pub fn bundle_files(files: &[Vec<u8>]) -> Vec<u8> {
        let mut res = Vec::new();

        let drop_files = DROPFILES {
            pFiles: size_of::<DROPFILES>() as u32,
            pt: POINT { x: 0, y: 0 },
            fNC: false.into(),
            fWide: true.into(),
        };

        let drop_files = unsafe { as_u8_slice(&drop_files) };
        res.extend_from_slice(drop_files);

        for f in files {
            res.extend_from_slice(f);
        }
        res.extend_from_slice(&[0, 0]);

        res
    }

    fn data_for_hdrop(&self) -> Option<Vec<u8>> {
        let n_items = self.writer.data.items.len();
        let files: Vec<_> = (0..n_items)
            .filter_map(|i| self.data_for_format(CF_HDROP.0 as u32, i))
            .collect();
        if files.is_empty() {
            None
        } else {
            Some(Self::bundle_files(&files))
        }
    }

    fn get_formats(&self) -> Vec<FORMATETC> {
        let first_item = self.writer.data.items.first();
        let mut res = Vec::<FORMATETC>::new();
        if let Some(item) = first_item {
            for representation in &item.representations {
                match representation {
                    DataSourceItemRepresentation::Simple { formats, data: _ } => {
                        for ty in formats {
                            let format = format_from_string(ty);
                            res.push(make_format_with_tymed(format, TYMED_HGLOBAL));
                        }
                    }
                    DataSourceItemRepresentation::Lazy { formats, id: _ } => {
                        for ty in formats {
                            let format = format_from_string(ty);
                            res.push(make_format_with_tymed(format, TYMED_HGLOBAL));
                        }
                    }
                    _ => {}
                }
            }
        }
        res
    }
}

impl Drop for DataObject {
    fn drop(&mut self) {
        // self.run_loop.stop();
    }
}

impl IDataObject_Impl for DataObject {
    fn GetData(
        &self,
        pformatetcin: *const windows::Win32::System::Com::FORMATETC,
    ) -> windows::core::Result<windows::Win32::System::Com::STGMEDIUM> {
        let format = unsafe { &*pformatetcin };
        let data = if format.cfFormat as u32 == CF_HDROP.0 {
            self.data_for_hdrop()
        } else {
            self.data_for_format(format.cfFormat as u32, 0)
        };

        match data {
            Some(data) => {
                if (format.tymed & TYMED_HGLOBAL.0 as u32) != 0 {
                    let global = self.global_from_data(&data)?;
                    Ok(STGMEDIUM {
                        tymed: TYMED_HGLOBAL.0 as u32,
                        Anonymous: STGMEDIUM_0 { hGlobal: global },
                        pUnkForRelease: None,
                    })
                } else if (format.tymed & TYMED_ISTREAM.0 as u32) != 0 {
                    let stream = unsafe { SHCreateMemStream(data.as_ptr(), data.len() as u32) };
                    let stream =
                        stream.ok_or_else(|| windows::core::Error::from(DV_E_FORMATETC))?;
                    unsafe {
                        stream.Seek(0, STREAM_SEEK_END)?;
                    }
                    Ok(STGMEDIUM {
                        tymed: TYMED_ISTREAM.0 as u32,
                        Anonymous: STGMEDIUM_0 {
                            pstm: ManuallyDrop::new(Some(stream)),
                        },
                        pUnkForRelease: None,
                    })
                } else {
                    Err(DV_E_FORMATETC.into())
                }
            }
            None => Err(DV_E_FORMATETC.into()),
        }
    }

    fn GetDataHere(
        &self,
        _pformatetc: *const windows::Win32::System::Com::FORMATETC,
        _pmedium: *mut windows::Win32::System::Com::STGMEDIUM,
    ) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn QueryGetData(
        &self,
        pformatetc: *const windows::Win32::System::Com::FORMATETC,
    ) -> windows::core::Result<()> {
        let format = unsafe { &*pformatetc };
        let index = self.get_formats().iter().position(|e| {
            e.cfFormat == format.cfFormat
                && (e.tymed & format.tymed) != 0
                && e.dwAspect == format.dwAspect
                && e.lindex == format.lindex
        });
        match index {
            Some(_) => Ok(()),
            None => Err(S_FALSE.into()),
        }
    }

    fn GetCanonicalFormatEtc(
        &self,
        _pformatectin: *const windows::Win32::System::Com::FORMATETC,
    ) -> windows::core::Result<windows::Win32::System::Com::FORMATETC> {
        Err(DATA_S_SAMEFORMATETC.into())
    }

    fn SetData(
        &self,
        _pformatetc: *const windows::Win32::System::Com::FORMATETC,
        _pmedium: *const windows::Win32::System::Com::STGMEDIUM,
        _frelease: windows::Win32::Foundation::BOOL,
    ) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn EnumFormatEtc(
        &self,
        dwdirection: u32,
    ) -> windows::core::Result<windows::Win32::System::Com::IEnumFORMATETC> {
        if dwdirection == DATADIR_GET.0 as u32 {
            unsafe { SHCreateStdEnumFmtEtc(&self.get_formats()) }
        } else {
            Err(E_NOTIMPL.into())
        }
    }

    fn DAdvise(
        &self,
        _pformatetc: *const windows::Win32::System::Com::FORMATETC,
        _advf: u32,
        _padvsink: &core::option::Option<windows::Win32::System::Com::IAdviseSink>,
    ) -> windows::core::Result<u32> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }

    fn DUnadvise(&self, _dwconnection: u32) -> windows::core::Result<()> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }

    fn EnumDAdvise(&self) -> windows::core::Result<windows::Win32::System::Com::IEnumSTATDATA> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }
}
