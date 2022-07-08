use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    mem::{size_of, ManuallyDrop},
    rc::Rc,
    slice,
    sync::Arc,
};

use windows::{
    core::{implement, HRESULT},
    Win32::{
        Foundation::{
            BOOL, DATA_S_SAMEFORMATETC, DV_E_FORMATETC, E_NOTIMPL, E_OUTOFMEMORY,
            OLE_E_ADVISENOTSUPPORTED, POINT, S_FALSE,
        },
        System::{
            Com::{
                IBindCtx, IDataObject, IDataObject_Impl, IStream, DATADIR_GET, FORMATETC,
                STGMEDIUM, STGMEDIUM_0, STREAM_SEEK_END, TYMED_HGLOBAL, TYMED_ISTREAM,
            },
            DataExchange::RegisterClipboardFormatW,
            Memory::{
                GlobalAlloc, GlobalFree, GlobalLock, GlobalSize, GlobalUnlock, GLOBAL_ALLOC_FLAGS,
            },
            Ole::ReleaseStgMedium,
            SystemServices::CF_HDROP,
        },
        UI::Shell::{
            IDataObjectAsyncCapability, IDataObjectAsyncCapability_Impl, SHCreateMemStream,
            SHCreateStdEnumFmtEtc, CFSTR_FILECONTENTS, CFSTR_FILEDESCRIPTOR, DROPFILES,
            FD_ATTRIBUTES, FD_PROGRESSUI, FILEDESCRIPTORW,
        },
    },
};

use crate::{
    api_model::{DataSourceItemRepresentation, DataSourceValueId, VirtualFileStorage},
    util::DropNotifier,
    value_coerce::{CoerceToData, StringFormat},
    value_promise::ValuePromiseResult,
};

use super::{
    common::{
        as_u8_slice, format_from_string, format_to_string, make_format_with_tymed,
        make_format_with_tymed_index, message_loop_hwnds, pump_message_loop,
    },
    MyStream, PlatformDataSource,
};

const DATA_E_FORMATETC: HRESULT = HRESULT(-2147221404 + 1);

#[implement(IDataObject, IDataObjectAsyncCapability)]
pub struct DataObject {
    data_source: Rc<PlatformDataSource>,
    _drop_notifier: Arc<DropNotifier>,
    extra_data: RefCell<HashMap<u16, Vec<u8>>>,
    in_operation: Cell<bool>, // async stream
}

impl DataObject {
    pub fn create(
        data_source: Rc<PlatformDataSource>,
        drop_notifier: Arc<DropNotifier>,
    ) -> IDataObject {
        let data_object = Self {
            data_source,
            _drop_notifier: drop_notifier,
            extra_data: RefCell::new(HashMap::new()),
            in_operation: Cell::new(false),
        };
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
        let delegate = self.data_source.delegate.upgrade();
        if let Some(delegate) = delegate {
            let hwnds = message_loop_hwnds();
            let data = delegate.get_lazy_data(self.data_source.isolate_id, id, format, None);
            loop {
                match data.try_take() {
                    Some(ValuePromiseResult::Ok { value }) => {
                        return value.coerce_to_data(StringFormat::Utf16NullTerminated)
                    }
                    Some(ValuePromiseResult::Cancelled) => return None,
                    None => pump_message_loop(&hwnds),
                }
            }
        } else {
            None
        }
    }

    fn data_for_format(&self, format: u32, index: usize) -> Option<Vec<u8>> {
        let item = self.data_source.data.items.get(index);
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
        let n_items = self.data_source.data.items.len();
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
        let mut res = Vec::<_>::new();
        let mut index = 0;
        // Put virtual files first
        for item in &self.data_source.data.items {
            for repr in &item.representations {
                if repr.is_virtual_file() {
                    if index == 0 {
                        res.push(make_format_with_tymed(
                            unsafe { RegisterClipboardFormatW(CFSTR_FILEDESCRIPTOR) },
                            TYMED_HGLOBAL,
                        ));
                    }
                    res.push(make_format_with_tymed_index(
                        unsafe { RegisterClipboardFormatW(CFSTR_FILECONTENTS) },
                        TYMED_ISTREAM,
                        index,
                    ));
                    index += 1;
                }
            }
        }
        // Regular and lazy items second
        let first_item = self.data_source.data.items.first();
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
        // Extra data (set through SetData) last
        let extra_data = self.extra_data.borrow();
        for format in extra_data.keys() {
            res.push(make_format_with_tymed(*format as u32, TYMED_HGLOBAL));
            res.push(make_format_with_tymed(*format as u32, TYMED_ISTREAM));
        }
        res
    }

    fn file_descriptor_for_item(file_name: &str) -> FILEDESCRIPTORW {
        let mut name_buf: [u16; 260] = [0; 260];
        let mut name_str: Vec<_> = file_name.encode_utf16().collect();
        name_str.truncate(name_buf.len() - 1);
        name_buf[0..name_str.len()].copy_from_slice(&name_str);
        FILEDESCRIPTORW {
            dwFlags: (FD_ATTRIBUTES.0 | FD_PROGRESSUI.0) as u32,
            cFileName: name_buf,
            ..FILEDESCRIPTORW::default()
        }
    }

    fn data_for_file_group_descritor(&self) -> Option<Vec<u8>> {
        let mut cnt = 0;
        let mut descriptors = Vec::<FILEDESCRIPTORW>::new();
        for item in &self.data_source.data.items {
            if item.representations.iter().any(|a| a.is_virtual_file()) {
                cnt += 1;
                let name = item
                    .suggested_name
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| format!("File {}", cnt));
                descriptors.push(Self::file_descriptor_for_item(&name));
            }
        }
        let mut res = Vec::new();
        let len = descriptors.len() as u32;
        if len == 0 {
            return None;
        }
        res.extend_from_slice(unsafe { as_u8_slice(&len) });
        for descriptor in &descriptors {
            res.extend_from_slice(unsafe { as_u8_slice(descriptor) });
        }
        Some(res)
    }

    fn stream_for_virtual_file(
        &self,
        id: DataSourceValueId,
        storage_suggestion: &Option<VirtualFileStorage>,
    ) -> Option<IStream> {
        if let Some(delegate) = self.data_source.delegate.upgrade() {
            // delegate.get_virtual_file(isolate_id, virtual_file_id, stream_handle, on_size_known, on_progress, on_done)
        }
        Some(MyStream::create_on_another_thread())
    }

    fn stream_for_virtual_file_index(&self, mut index: usize) -> Option<IStream> {
        for item in &self.data_source.data.items {
            if index > 0 && item.representations.iter().any(|r| r.is_virtual_file()) {
                index -= 1;
                continue;
            }
            for repr in &item.representations {
                if let DataSourceItemRepresentation::VirtualFile {
                    id,
                    format: _,
                    storage_suggestion,
                } = repr
                {
                    return self.stream_for_virtual_file(*id, storage_suggestion);
                }
            }
        }
        None
    }
}

impl IDataObject_Impl for DataObject {
    fn GetData(
        &self,
        pformatetcin: *const windows::Win32::System::Com::FORMATETC,
    ) -> windows::core::Result<windows::Win32::System::Com::STGMEDIUM> {
        let format = unsafe { &*pformatetcin };
        let format_file_descriptor = unsafe { RegisterClipboardFormatW(CFSTR_FILEDESCRIPTOR) };
        let format_file_contents = unsafe { RegisterClipboardFormatW(CFSTR_FILECONTENTS) };

        if format.cfFormat as u32 == format_file_contents {
            let stream = self.stream_for_virtual_file_index(format.lindex as usize);
            return Ok(STGMEDIUM {
                tymed: TYMED_ISTREAM.0 as u32,
                Anonymous: STGMEDIUM_0 {
                    pstm: ManuallyDrop::new(stream),
                },
                pUnkForRelease: None,
            });
        }

        let data = self
            .extra_data
            .borrow()
            .get(&format.cfFormat)
            .cloned()
            .or_else(|| {
                if format.cfFormat as u32 == format_file_descriptor {
                    self.data_for_file_group_descritor()
                } else if format.cfFormat as u32 == CF_HDROP.0 {
                    self.data_for_hdrop()
                } else {
                    self.data_for_format(format.cfFormat as u32, 0)
                }
            });
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
            None => {
                // possibly extra data
                if (format.tymed == TYMED_HGLOBAL.0 as u32
                    || format.tymed == TYMED_ISTREAM.0 as u32)
                    && self.extra_data.borrow().contains_key(&format.cfFormat)
                {
                    Ok(())
                } else {
                    Err(S_FALSE.into())
                }
            }
        }
    }

    fn GetCanonicalFormatEtc(
        &self,
        _pformatectin: *const FORMATETC,
        _pformatetcout: *mut FORMATETC,
    ) -> ::windows::core::HRESULT {
        DATA_S_SAMEFORMATETC
    }

    fn SetData(
        &self,
        pformatetc: *const windows::Win32::System::Com::FORMATETC,
        pmedium: *const windows::Win32::System::Com::STGMEDIUM,
        frelease: windows::Win32::Foundation::BOOL,
    ) -> windows::core::Result<()> {
        let format = unsafe { &*pformatetc };
        if format.tymed == TYMED_HGLOBAL.0 as u32 {
            unsafe {
                let medium = &*pmedium;
                let size = GlobalSize(medium.Anonymous.hGlobal);
                let global_data = GlobalLock(medium.Anonymous.hGlobal);

                let v = slice::from_raw_parts(global_data as *const u8, size);
                let global_data: Vec<u8> = v.into();

                GlobalUnlock(medium.Anonymous.hGlobal);
                self.extra_data
                    .borrow_mut()
                    .insert(format.cfFormat, global_data);

                if frelease.as_bool() {
                    ReleaseStgMedium(pmedium as *mut _);
                }
            }

            Ok(())
        } else if format.tymed == TYMED_ISTREAM.0 as u32 {
            unsafe {
                let medium = &*pmedium;
                let stream = medium.Anonymous.pstm.as_ref().cloned();
                let mut stream_data = Vec::<u8>::new();
                let mut buf: [u8; 4096] = [0; 4096];
                if let Some(stream) = stream {
                    loop {
                        let mut num_read: u32 = 0;
                        if stream
                            .Read(
                                buf.as_mut_ptr() as *mut _,
                                buf.len() as u32,
                                &mut num_read as *mut _,
                            )
                            .is_err()
                        {
                            break;
                        }

                        if num_read == 0 {
                            break;
                        }
                        stream_data.extend_from_slice(&buf[..num_read as usize]);
                    }
                }

                self.extra_data
                    .borrow_mut()
                    .insert(format.cfFormat, stream_data);

                if frelease.as_bool() {
                    ReleaseStgMedium(pmedium as *mut _);
                }
            }

            Ok(())
        } else {
            Err(DATA_E_FORMATETC.into())
        }
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

impl IDataObjectAsyncCapability_Impl for DataObject {
    fn SetAsyncMode(&self, _fdoopasync: BOOL) -> windows::core::Result<()> {
        Ok(())
    }

    fn GetAsyncMode(&self) -> windows::core::Result<BOOL> {
        Ok(true.into())
    }

    fn StartOperation(
        &self,
        _pbcreserved: &core::option::Option<IBindCtx>,
    ) -> windows::core::Result<()> {
        self.in_operation.replace(true);
        Ok(())
    }

    fn InOperation(&self) -> windows::core::Result<BOOL> {
        Ok(self.in_operation.get().into())
    }

    fn EndOperation(
        &self,
        hresult: windows::core::HRESULT,
        _pbcreserved: &core::option::Option<IBindCtx>,
        dweffects: u32,
    ) -> windows::core::Result<()> {
        println!("End operation {:?} ef {:?}", hresult, dweffects);
        self.in_operation.replace(false);
        Ok(())
    }
}