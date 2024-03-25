use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    mem::{size_of, ManuallyDrop},
    rc::Rc,
    slice,
    sync::Arc,
    time::Duration,
};

use irondash_message_channel::IsolateId;
use irondash_run_loop::{platform::PollSession, RunLoop};
use threadpool::ThreadPool;
use windows::{
    core::{implement, HRESULT, HSTRING},
    Win32::{
        Foundation::{
            GlobalFree, BOOL, DATA_S_SAMEFORMATETC, DV_E_FORMATETC, E_NOTIMPL, E_OUTOFMEMORY,
            HGLOBAL, OLE_E_ADVISENOTSUPPORTED, POINT, S_FALSE, S_OK,
        },
        System::{
            Com::{
                IAdviseSink, IBindCtx, IDataObject, IDataObject_Impl, IStream, DATADIR_GET,
                FORMATETC, STGMEDIUM, STGMEDIUM_0, STREAM_SEEK_END, STREAM_SEEK_SET, TYMED,
                TYMED_HGLOBAL, TYMED_ISTREAM,
            },
            DataExchange::RegisterClipboardFormatW,
            Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GLOBAL_ALLOC_FLAGS},
            Ole::{ReleaseStgMedium, CF_DIB, CF_DIBV5, CF_HDROP, DROPEFFECT},
        },
        UI::Shell::{
            IDataObjectAsyncCapability, IDataObjectAsyncCapability_Impl, SHCreateMemStream,
            SHCreateStdEnumFmtEtc, CFSTR_FILECONTENTS, CFSTR_FILEDESCRIPTOR,
            CFSTR_LOGICALPERFORMEDDROPEFFECT, CFSTR_PERFORMEDDROPEFFECT, DROPFILES, FD_ATTRIBUTES,
            FD_PROGRESSUI, FILEDESCRIPTORW,
        },
    },
};

use crate::{
    api_model::{DataProviderValueId, DataRepresentation, VirtualFileStorage},
    data_provider_manager::{DataProviderHandle, PlatformDataProviderDelegate, VirtualFileResult},
    log::OkLog,
    segmented_queue::{new_segmented_queue, QueueConfiguration},
    util::DropNotifier,
    value_coerce::{CoerceToData, StringFormat},
    value_promise::{Promise, ValuePromiseResult},
};

use super::{
    add_stream_entry,
    common::{
        as_u8_slice, format_from_string, format_to_string, make_format_with_tymed,
        make_format_with_tymed_index, read_stream_fully,
    },
    image_conversion::convert_to_dib,
    virtual_file_stream::{VirtualFileStream, VirtualStreamSession},
    PlatformDataProvider,
};

const DATA_E_FORMATETC: HRESULT = HRESULT(-2147221404 + 1);

struct ProviderEntry {
    provider: Rc<PlatformDataProvider>,
    _handle: Arc<DataProviderHandle>,
}

#[implement(IDataObject, IDataObjectAsyncCapability)]
pub struct DataObject {
    providers: Vec<ProviderEntry>,
    extra_data: RefCell<HashMap<u16, Vec<u8>>>,
    in_operation: Cell<bool>, // async stream
    virtual_stream_notifiers: RefCell<Vec<Arc<DropNotifier>>>,
    thread_pool: RefCell<Option<ThreadPool>>,
}

/// These formats are not commonly supported on Windows. If they
/// are present as payload, DataObject will provide on-demand
/// DIB and DIBV5 representation (unless the payload already contains
/// DIB or DIBV5)
static FOREIGN_IMAGE_FORMATS: &[&str] = &["PNG", "GIF", "JFIF"];

impl DataObject {
    pub fn create(
        providers: Vec<(Rc<PlatformDataProvider>, Arc<DataProviderHandle>)>,
    ) -> IDataObject {
        let data_object = Self {
            providers: providers
                .into_iter()
                .map(|p| ProviderEntry {
                    provider: p.0,
                    _handle: p.1,
                })
                .collect(),
            extra_data: RefCell::new(HashMap::new()),
            in_operation: Cell::new(false),
            virtual_stream_notifiers: RefCell::new(Vec::new()),
            thread_pool: RefCell::new(None),
        };
        data_object.into()
    }

    fn global_from_data(&self, data: &[u8]) -> windows::core::Result<HGLOBAL> {
        unsafe {
            let global = GlobalAlloc(GLOBAL_ALLOC_FLAGS(0), data.len())?;
            let global_data = GlobalLock(global);
            if global_data.is_null() {
                GlobalFree(global)?;
                Err(E_OUTOFMEMORY.into())
            } else {
                std::ptr::copy_nonoverlapping(data.as_ptr(), global_data as *mut u8, data.len());
                GlobalUnlock(global).ok();
                Ok(global)
            }
        }
    }

    fn lazy_data_for_id(
        &self,
        provider: &PlatformDataProvider,
        id: DataProviderValueId,
    ) -> Option<Vec<u8>> {
        let delegate = provider.delegate.upgrade();
        if let Some(delegate) = delegate {
            let data = delegate.get_lazy_data(provider.isolate_id, id, None);
            let mut poll_session = PollSession::new();
            loop {
                match data.try_take() {
                    Some(ValuePromiseResult::Ok { value }) => {
                        return value.coerce_to_data(StringFormat::Utf16NullTerminated)
                    }
                    Some(ValuePromiseResult::Cancelled) => return None,
                    None => RunLoop::current()
                        .platform_run_loop
                        .poll_once(&mut poll_session),
                }
            }
        } else {
            None
        }
    }

    fn data_for_format(&self, format: u32, index: usize) -> Option<Vec<u8>> {
        let provider = self.providers.get(index).as_ref().cloned();
        if let Some(provider) = provider {
            let provider = &provider.provider;
            let format_string = format_to_string(format);
            for representation in &provider.data.representations {
                match representation {
                    DataRepresentation::Simple { format, data } => {
                        if &format_string == format {
                            return data.coerce_to_data(StringFormat::Utf16NullTerminated);
                        }
                    }
                    DataRepresentation::Lazy { format, id } => {
                        if &format_string == format {
                            return self.lazy_data_for_id(provider, *id);
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

    /// Bundles slice of utf16 encoded string into CF_HDROP
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
        let n_items = self.providers.len();
        let files: Vec<_> = (0..n_items)
            .filter_map(|i| self.data_for_format(CF_HDROP.0 as u32, i))
            .collect();
        if files.is_empty() {
            None
        } else {
            Some(Self::bundle_files(&files))
        }
    }

    fn get_source_stream_for_synthesized_bitmap(&self) -> windows::core::Result<IStream> {
        let foreign_formats = Self::foreign_formats();
        let formats = self.get_formats();
        for format in formats {
            let format = format.cfFormat as u32;
            if foreign_formats.contains(&format) {
                return self.get_stream(format);
            }
        }
        Err(windows::core::Error::new(
            DATA_E_FORMATETC,
            "Did not find original image stream".into(),
        ))
    }

    fn synthesize_bitmap_data(&self, use_v5: bool) -> windows::core::Result<Vec<u8>> {
        let input_stream = self.get_source_stream_for_synthesized_bitmap()?;
        convert_to_dib(input_stream, use_v5)
    }

    fn foreign_formats() -> Vec<u32> {
        FOREIGN_IMAGE_FORMATS
            .iter()
            .map(|f| unsafe { RegisterClipboardFormatW(&HSTRING::from(*f)) })
            .collect()
    }

    /// If there are any image formats not supported by windows natively
    /// and no DIB or DIBV5 we need to generate those.
    fn needs_synthesize_bitmap(&self) -> bool {
        let foreign_formats = Self::foreign_formats();
        let mut has_bmp = false;
        let mut has_foreign = false;
        for provider in &self.providers {
            for repr in &provider.provider.data.representations {
                let repr_format = format_from_string(repr.format());
                has_bmp |= repr_format == CF_DIBV5.0 as u32 || repr_format == CF_DIB.0 as u32;
                has_foreign |= foreign_formats.contains(&repr_format);
            }
        }
        has_foreign && !has_bmp
    }

    fn get_formats(&self) -> Vec<FORMATETC> {
        let mut res = Vec::<_>::new();
        let mut index = 0;
        // Put virtual files first
        for provider in &self.providers {
            for repr in &provider.provider.data.representations {
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
        let first_provider = self.providers.first();
        if let Some(provider) = first_provider {
            for representation in &provider.provider.data.representations {
                match representation {
                    DataRepresentation::Simple { format, data: _ } => {
                        let format = format_from_string(format);
                        res.push(make_format_with_tymed(format, TYMED_HGLOBAL));
                    }
                    DataRepresentation::Lazy { format, id: _ } => {
                        let format = format_from_string(format);
                        res.push(make_format_with_tymed(format, TYMED_HGLOBAL));
                    }
                    _ => {}
                }
            }
        }

        if self.needs_synthesize_bitmap() {
            res.push(make_format_with_tymed(CF_DIB.0 as u32, TYMED_HGLOBAL));
            res.push(make_format_with_tymed(CF_DIBV5.0 as u32, TYMED_HGLOBAL));
        }

        // Extra data (set through SetData) last
        let extra_data = self.extra_data.borrow();
        for format in extra_data.keys() {
            res.push(make_format_with_tymed(
                *format as u32,
                TYMED(TYMED_HGLOBAL.0 | TYMED_ISTREAM.0),
            ));
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
        for provider in &self.providers {
            let data = &provider.provider.data;
            if data.representations.iter().any(|a| a.is_virtual_file()) {
                cnt += 1;
                let name = data
                    .suggested_name
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| format!("File {cnt}"));
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

    fn create_virtual_stream_session(
        delegate: Rc<dyn PlatformDataProviderDelegate>,
        isolate_id: IsolateId,
        virtual_file_id: DataProviderValueId,
        configuration: QueueConfiguration,
    ) -> VirtualStreamSession {
        let (writer, reader) = new_segmented_queue(configuration);
        let stream_handle = add_stream_entry(writer);
        let size_promise = Arc::new(Promise::<Option<i64>>::new());
        let size_promise_clone = size_promise.clone();
        let error_promise = Arc::new(Promise::<String>::new());
        let error_promise_clone = error_promise.clone();
        let session_handle = delegate.get_virtual_file(
            isolate_id,
            virtual_file_id,
            stream_handle,
            Box::new(move |size| size_promise_clone.set(size)),
            Box::new(move |_progress| {}),
            Box::new(move |result| {
                if let VirtualFileResult::Error { message } = result {
                    error_promise_clone.set(message);
                }
            }),
        );
        VirtualStreamSession {
            reader,
            size_promise,
            error_promise,
            handle: session_handle,
        }
    }

    fn stream_for_virtual_file(
        &self,
        provider: &PlatformDataProvider,
        virtual_file_id: DataProviderValueId,
        storage_suggestion: &Option<VirtualFileStorage>,
        agile: bool,
    ) -> Option<IStream> {
        if let Some(delegate) = provider.delegate.upgrade() {
            let storage_suggestion =
                storage_suggestion.unwrap_or(VirtualFileStorage::TemporaryFile);
            let configuration = if storage_suggestion == VirtualFileStorage::TemporaryFile {
                QueueConfiguration {
                    memory_segment_max_size: 1024 * 1024 * 4,
                    file_segment_max_length: 1024 * 1024 * 30,
                    max_memory_usage: Some(1024 * 1024 * 12),
                }
            } else {
                QueueConfiguration {
                    memory_segment_max_size: 1024 * 1024 * 4,
                    file_segment_max_length: 0,
                    max_memory_usage: None,
                }
            };
            let isolate_id = provider.isolate_id;
            let provider = move || {
                Self::create_virtual_stream_session(
                    delegate,
                    isolate_id,
                    virtual_file_id,
                    configuration,
                )
            };
            let (stream, notifier) = if agile {
                VirtualFileStream::create_agile(provider)
            } else {
                let mut thread_pool = self.thread_pool.borrow_mut();
                let thread_pool = thread_pool.get_or_insert_with(|| ThreadPool::new(1));
                VirtualFileStream::create_marshalled_on_background_thread(provider, thread_pool)
            };

            // The drop notifier will be invoked when DataObject gets released
            // That will ensure that the stream is destroyed when data object
            // is dropped in case the client leaks the stream.
            self.virtual_stream_notifiers.borrow_mut().push(notifier);
            Some(stream)
        } else {
            None
        }
    }

    fn stream_for_virtual_file_index(&self, mut index: usize, agile: bool) -> Option<IStream> {
        for provider in &self.providers {
            let provider = &provider.provider;
            // Skip all virtual files before the requested one.
            if index > 0
                && provider
                    .data
                    .representations
                    .iter()
                    .any(|r| r.is_virtual_file())
            {
                index -= 1;
                continue;
            }
            for repr in &provider.data.representations {
                if let DataRepresentation::VirtualFile {
                    id,
                    format: _,
                    storage_suggestion,
                } = repr
                {
                    return self.stream_for_virtual_file(provider, *id, storage_suggestion, agile);
                }
            }
        }
        None
    }
}

thread_local! {
    static IS_LOCAL_REQUEST: Cell<bool> = const { Cell::new(false) };
}

impl DataObject {
    pub fn with_local_request<T, F: FnOnce() -> T>(f: F) -> T {
        let prev = IS_LOCAL_REQUEST.with(|a| a.replace(true));
        let res = f();
        IS_LOCAL_REQUEST.with(|a| a.set(prev));
        res
    }

    fn is_local_request() -> bool {
        IS_LOCAL_REQUEST.with(|f| f.get())
    }
}

impl Drop for DataObject {
    fn drop(&mut self) {
        // Keep the streams alive for one second after disposing data object
        // to give the client chance to interact with stream.
        // Otherwise the streams will be disposed to prevent leaks.
        // See VirtualFileStream::dispose()
        let notifiers: Vec<_> = self
            .virtual_stream_notifiers
            .borrow_mut()
            .drain(0..)
            .collect();
        RunLoop::current()
            .schedule(Duration::from_secs(1), move || {
                let _notifiers = notifiers;
            })
            .detach();
    }
}

#[allow(non_snake_case)]
impl IDataObject_Impl for DataObject {
    fn GetData(&self, pformatetcin: *const FORMATETC) -> windows::core::Result<STGMEDIUM> {
        let format = unsafe { &*pformatetcin };
        let format_file_descriptor = unsafe { RegisterClipboardFormatW(CFSTR_FILEDESCRIPTOR) };
        let format_file_contents = unsafe { RegisterClipboardFormatW(CFSTR_FILECONTENTS) };

        if format.cfFormat as u32 == format_file_contents {
            let stream = self
                .stream_for_virtual_file_index(format.lindex as usize, Self::is_local_request());
            return Ok(STGMEDIUM {
                tymed: TYMED_ISTREAM.0 as u32,
                u: STGMEDIUM_0 {
                    pstm: ManuallyDrop::new(stream),
                },
                pUnkForRelease: ManuallyDrop::new(None),
            });
        }

        let needs_generate_bitmap = self.needs_synthesize_bitmap();

        let data = self
            .extra_data
            .borrow()
            .get(&format.cfFormat)
            .cloned()
            .or_else(|| {
                if format.cfFormat as u32 == format_file_descriptor {
                    self.data_for_file_group_descritor()
                } else if format.cfFormat == CF_HDROP.0 {
                    self.data_for_hdrop()
                } else if needs_generate_bitmap && format.cfFormat == CF_DIB.0 {
                    self.synthesize_bitmap_data(false).ok_log()
                } else if needs_generate_bitmap && format.cfFormat == CF_DIBV5.0 {
                    self.synthesize_bitmap_data(true).ok_log()
                } else {
                    self.data_for_format(format.cfFormat as u32, 0)
                }
            });

        // println!("DATA {:?} {:?}", data, format_to_string(format.cfFormat as u32));

        match data {
            Some(data) => {
                if (format.tymed & TYMED_HGLOBAL.0 as u32) != 0 {
                    let global = self.global_from_data(&data)?;
                    Ok(STGMEDIUM {
                        tymed: TYMED_HGLOBAL.0 as u32,
                        u: STGMEDIUM_0 { hGlobal: global },
                        pUnkForRelease: ManuallyDrop::new(None),
                    })
                } else if (format.tymed & TYMED_ISTREAM.0 as u32) != 0 {
                    let stream = unsafe { SHCreateMemStream(Some(&data)) };
                    let stream =
                        stream.ok_or_else(|| windows::core::Error::from(DV_E_FORMATETC))?;
                    unsafe {
                        stream.Seek(0, STREAM_SEEK_END, None)?;
                    }
                    Ok(STGMEDIUM {
                        tymed: TYMED_ISTREAM.0 as u32,
                        u: STGMEDIUM_0 {
                            pstm: ManuallyDrop::new(Some(stream)),
                        },
                        pUnkForRelease: ManuallyDrop::new(None),
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
    ) -> windows::core::HRESULT {
        let format = unsafe { &*pformatetc };
        let index = self.get_formats().iter().position(|e| {
            e.cfFormat == format.cfFormat
                && (e.tymed & format.tymed) != 0
                && e.dwAspect == format.dwAspect
                && e.lindex == format.lindex
        });
        match index {
            Some(_) => S_OK,
            None => {
                // possibly extra data
                if (format.tymed == TYMED_HGLOBAL.0 as u32
                    || format.tymed == TYMED_ISTREAM.0 as u32)
                    && self.extra_data.borrow().contains_key(&format.cfFormat)
                {
                    S_OK
                } else {
                    S_FALSE
                }
            }
        }
    }

    fn GetCanonicalFormatEtc(
        &self,
        pformatectin: *const FORMATETC,
        pformatetcout: *mut FORMATETC,
    ) -> ::windows::core::HRESULT {
        let fmt_out = unsafe { &mut *pformatetcout };
        let fmt_in = unsafe { &*pformatectin };
        *fmt_out = *fmt_in;
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
                let size = GlobalSize(medium.u.hGlobal);
                let global_data = GlobalLock(medium.u.hGlobal);

                let v = slice::from_raw_parts(global_data as *const u8, size);
                let global_data: Vec<u8> = v.into();

                GlobalUnlock(medium.u.hGlobal).ok();
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
                let stream = medium.u.pstm.as_ref().cloned();

                let stream_data = if let Some(stream) = stream {
                    stream.Seek(0, STREAM_SEEK_SET, None)?;
                    read_stream_fully(&stream)?
                } else {
                    Vec::new()
                };

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
        _pformatetc: *const FORMATETC,
        _advf: u32,
        _padvsink: Option<&IAdviseSink>,
    ) -> ::windows::core::Result<u32> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }

    fn DUnadvise(&self, _dwconnection: u32) -> windows::core::Result<()> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }

    fn EnumDAdvise(&self) -> windows::core::Result<windows::Win32::System::Com::IEnumSTATDATA> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }
}

#[allow(non_snake_case)]
impl IDataObjectAsyncCapability_Impl for DataObject {
    fn SetAsyncMode(&self, _fdoopasync: BOOL) -> windows::core::Result<()> {
        Ok(())
    }

    fn GetAsyncMode(&self) -> windows::core::Result<BOOL> {
        Ok(true.into())
    }

    fn StartOperation(&self, _pbcreserved: Option<&IBindCtx>) -> windows::core::Result<()> {
        self.in_operation.replace(true);
        Ok(())
    }

    fn InOperation(&self) -> windows::core::Result<BOOL> {
        Ok(self.in_operation.get().into())
    }

    fn EndOperation(
        &self,
        _hresult: windows::core::HRESULT,
        _pbcreserved: Option<&IBindCtx>,
        _dweffects: u32,
    ) -> windows::core::Result<()> {
        self.in_operation.replace(false);
        Ok(())
    }
}

pub trait GetData {
    unsafe fn do_get_data(
        &self,
        pformatetcin: *const FORMATETC,
    ) -> windows::core::Result<STGMEDIUM>;

    unsafe fn do_query_get_data(&self, format: *const FORMATETC) -> HRESULT;

    fn get_data(&self, format: u32) -> windows::core::Result<Vec<u8>> {
        let format = make_format_with_tymed(format, TYMED(TYMED_ISTREAM.0 | TYMED_HGLOBAL.0));

        unsafe {
            let mut medium = self.do_get_data(&format as *const _)?;
            let res = if medium.tymed == TYMED_ISTREAM.0 as u32 {
                let stream = medium.u.pstm.as_ref().cloned();
                if let Some(stream) = stream {
                    // IDataObject streams need to be rewound
                    stream.Seek(0, STREAM_SEEK_SET, None)?;
                    read_stream_fully(&stream)
                } else {
                    Ok(Vec::new())
                }
            } else if medium.tymed == TYMED_HGLOBAL.0 as u32 {
                let size = GlobalSize(medium.u.hGlobal);
                let data = GlobalLock(medium.u.hGlobal);

                let v = slice::from_raw_parts(data as *const u8, size);
                let res: Vec<u8> = v.into();

                GlobalUnlock(medium.u.hGlobal).ok();

                Ok(res)
            } else {
                Err(DATA_E_FORMATETC.into())
            };
            ReleaseStgMedium(&mut medium as *mut STGMEDIUM);
            res
        }
    }

    fn get_stream(&self, format: u32) -> windows::core::Result<IStream> {
        let format = make_format_with_tymed(format, TYMED(TYMED_ISTREAM.0 | TYMED_HGLOBAL.0));
        let res = unsafe {
            let mut medium = self.do_get_data(&format as *const _)?;
            let res = if medium.tymed == TYMED_ISTREAM.0 as u32 {
                medium.u.pstm.as_ref().cloned()
            } else if medium.tymed == TYMED_HGLOBAL.0 as u32 {
                let size = GlobalSize(medium.u.hGlobal);
                let data = GlobalLock(medium.u.hGlobal);
                let data = slice::from_raw_parts(data as *const u8, size);
                let res = SHCreateMemStream(Some(data));
                GlobalUnlock(medium.u.hGlobal).ok();
                res
            } else {
                None
            };
            ReleaseStgMedium(&mut medium as *mut STGMEDIUM);
            res
        };
        res.ok_or_else(|| DATA_E_FORMATETC.into())
    }

    fn has_data_for_format(&self, format: &FORMATETC) -> bool {
        unsafe { self.do_query_get_data(format as *const _) == S_OK }
    }

    fn has_data(&self, format: u32) -> bool {
        let format = make_format_with_tymed(format, TYMED_HGLOBAL);
        self.has_data_for_format(&format)
    }
}

pub trait DataObjectExt {
    fn performed_drop_effect(&self) -> Option<DROPEFFECT>;
}

impl GetData for IDataObject {
    unsafe fn do_get_data(
        &self,
        pformatetcin: *const FORMATETC,
    ) -> windows::core::Result<STGMEDIUM> {
        self.GetData(pformatetcin)
    }

    unsafe fn do_query_get_data(&self, format: *const FORMATETC) -> HRESULT {
        let res = self.QueryGetData(format);
        // Workaround for Windows Explorer:
        // When pasting data from Explorer QueryGetData will return DV_E_FORMATETC
        // for CFSTR_FILECONTENTS when index is specified in format.
        // This only affects QueryGetData, actually getting the data works as
        // expected.
        // https://github.com/superlistapp/super_native_extensions/issues/86
        if res == DV_E_FORMATETC {
            let format = unsafe { *format };
            if format.lindex != -1
                && format.cfFormat as u32 == unsafe { RegisterClipboardFormatW(CFSTR_FILECONTENTS) }
            {
                let format = FORMATETC {
                    cfFormat: format.cfFormat,
                    ptd: format.ptd,
                    dwAspect: format.dwAspect,
                    lindex: -1,
                    tymed: format.tymed,
                };
                return self.QueryGetData(&format as *const _);
            }
        }
        res
    }
}

impl GetData for DataObject {
    unsafe fn do_get_data(
        &self,
        pformatetcin: *const FORMATETC,
    ) -> windows::core::Result<STGMEDIUM> {
        self.GetData(pformatetcin)
    }

    unsafe fn do_query_get_data(&self, format: *const FORMATETC) -> HRESULT {
        match self.GetData(format) {
            Ok(_) => S_OK,
            Err(e) => e.into(),
        }
    }

    fn has_data_for_format(&self, format: &FORMATETC) -> bool {
        format.tymed == TYMED_HGLOBAL.0 as u32
            && format.lindex == 0
            && self.has_data(format.cfFormat as u32)
    }
}

impl<T> DataObjectExt for T
where
    T: GetData,
{
    fn performed_drop_effect(&self) -> Option<DROPEFFECT> {
        let format = unsafe { RegisterClipboardFormatW(CFSTR_PERFORMEDDROPEFFECT) };
        let logical_format = unsafe { RegisterClipboardFormatW(CFSTR_LOGICALPERFORMEDDROPEFFECT) };
        let data = self
            .get_data(logical_format)
            .ok()
            .or_else(|| self.get_data(format).ok());

        if let Some(data) = data {
            if data.len() == 4 {
                return Some(DROPEFFECT(u32::from_ne_bytes(data.try_into().unwrap())));
            }
        }

        None
    }
}
