use std::{
    ffi::CStr,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    rc::{Rc, Weak},
    slice,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};

use async_trait::async_trait;
use byte_slice_cast::AsSliceOf;
use irondash_message_channel::Value;
use irondash_run_loop::{
    util::{Capsule, FutureCompleter},
    RunLoop, RunLoopSender,
};
use rand::{distributions::Alphanumeric, Rng};
use threadpool::ThreadPool;
use url::Url;
use windows::{
    core::HSTRING,
    w,
    Win32::{
        Foundation::S_OK,
        Storage::FileSystem::{
            SetFileAttributesW, FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_HIDDEN,
            FILE_ATTRIBUTE_TEMPORARY,
        },
        System::{
            Com::{
                IDataObject, IStream, STATFLAG_NONAME, STATSTG, STGMEDIUM, STREAM_SEEK_SET, TYMED,
                TYMED_HGLOBAL, TYMED_ISTREAM,
            },
            DataExchange::RegisterClipboardFormatW,
            Memory::{GlobalLock, GlobalSize, GlobalUnlock},
            Ole::{OleGetClipboard, ReleaseStgMedium, CF_DIB, CF_DIBV5, CF_HDROP, CF_TIFF},
        },
        UI::Shell::{
            SHCreateMemStream, CFSTR_FILECONTENTS, CFSTR_FILEDESCRIPTOR, DROPFILES,
            FILEDESCRIPTORW, FILEGROUPDESCRIPTORW,
        },
    },
};

use crate::{
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    platform_impl::platform::common::make_format_with_tymed_index,
    reader_manager::{ReadProgress, VirtualFileReader},
    util::{get_target_path, DropNotifier, Movable},
};

use super::{
    common::{extract_formats, format_from_string, format_to_string},
    data_object::{DataObject, GetData},
    image_conversion::convert_to_png,
};

pub struct PlatformDataReader {
    data_object: IDataObject,
    _drop_notifier: Option<Arc<DropNotifier>>,
}

/// Virtual file descriptor
#[derive(Clone)]
struct FileDescriptor {
    name: String,
    format: String,
}

impl PlatformDataReader {
    pub async fn get_format_for_file_uri(
        file_uri: String,
    ) -> NativeExtensionsResult<Option<String>> {
        let url = Url::from_str(&file_uri)
            .map_err(|_| NativeExtensionsError::OtherError("Couldn't parse file URL".into()))?;
        let name = url.path_segments().and_then(|s| s.last());
        match name {
            Some(name) => {
                let format = mime_from_name(name);
                let format = mime_to_windows(format);
                Ok(Some(format))
            }
            None => Ok(None),
        }
    }

    pub fn get_items_sync(&self) -> NativeExtensionsResult<Vec<i64>> {
        Ok((0..self.item_count()? as i64).collect())
    }

    pub async fn get_items(&self) -> NativeExtensionsResult<Vec<i64>> {
        self.get_items_sync()
    }

    fn item_count(&self) -> NativeExtensionsResult<usize> {
        let descriptor_len = self.get_file_descriptors()?.map(|f| f.len()).unwrap_or(0);
        let hdrop_len = self.get_hdrop()?.map(|f| f.len()).unwrap_or(0);
        let file_len = descriptor_len.max(hdrop_len);
        if file_len > 0 {
            Ok(file_len)
        } else if !self.data_object_formats()?.is_empty() {
            Ok(1)
        } else {
            Ok(0)
        }
    }

    /// Returns formats that DataObject can provide.
    fn data_object_formats_raw(&self) -> NativeExtensionsResult<Vec<u32>> {
        let formats = extract_formats(&self.data_object)?
            .iter()
            .filter_map(|f| {
                if (f.tymed & TYMED_HGLOBAL.0 as u32) != 0
                    || (f.tymed & TYMED_ISTREAM.0 as u32) != 0
                {
                    Some(f.cfFormat as u32)
                } else {
                    None
                }
            })
            .collect();
        Ok(formats)
    }

    fn need_to_synthetize_png(&self) -> NativeExtensionsResult<bool> {
        let png = unsafe { RegisterClipboardFormatW(w!("PNG")) };
        let formats = self.data_object_formats_raw()?;
        let has_dib =
            formats.contains(&(CF_DIBV5.0 as u32)) || formats.contains(&(CF_DIB.0 as u32));
        let has_png = formats.contains(&png);
        Ok(has_dib && !has_png)
    }

    fn data_object_formats(&self) -> NativeExtensionsResult<Vec<u32>> {
        let mut res = self.data_object_formats_raw()?;
        if self.need_to_synthetize_png()? {
            let png = unsafe { RegisterClipboardFormatW(w!("PNG")) };
            res.push(png);
        }
        Ok(res)
    }

    pub fn get_formats_for_item_sync(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        let mut formats = if item == 0 {
            self.data_object_formats()?
                .iter()
                .map(|f| format_to_string(*f))
                .collect()
        } else if item > 0 {
            let hdrop_len = self.get_hdrop()?.map(|v| v.len()).unwrap_or(0);
            if item < hdrop_len as i64 {
                vec![format_to_string(CF_HDROP.0 as u32)]
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let descriptors = self.get_file_descriptors()?;
        if let Some(descriptors) = descriptors {
            if let Some(descriptor) = descriptors.get(item as usize) {
                // make virtual file highest priority
                formats.insert(0, descriptor.format.clone());
            }
        }

        Ok(formats)
    }

    pub async fn get_formats_for_item(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        self.get_formats_for_item_sync(item)
    }

    pub fn item_format_is_synthetized(
        &self,
        _item: i64,
        format: &str,
    ) -> NativeExtensionsResult<bool> {
        Ok(format == "PNG" && self.need_to_synthetize_png()?)
    }

    pub async fn can_copy_virtual_file_for_item(
        &self,
        item: i64,
        format: &str,
    ) -> NativeExtensionsResult<bool> {
        let descriptors = self.get_file_descriptors()?;
        if let Some(descriptors) = descriptors {
            if let Some(descriptor) = descriptors.get(item as usize) {
                return Ok(descriptor.format == format);
            }
        }
        Ok(false)
    }

    pub async fn can_read_virtual_file_for_item(
        &self,
        item: i64,
        format: &str,
    ) -> NativeExtensionsResult<bool> {
        self.can_copy_virtual_file_for_item(item, format).await
    }

    pub async fn get_suggested_name_for_item(
        &self,
        item: i64,
    ) -> NativeExtensionsResult<Option<String>> {
        let item = item as usize;
        if let Some(descriptors) = self.get_file_descriptors()? {
            if let Some(descriptor) = descriptors.get(item) {
                return Ok(Some(descriptor.name.clone()));
            }
        }
        if let Some(hdrop) = self.get_hdrop()? {
            if let Some(hdrop) = hdrop.get(item) {
                let path = Path::new(&hdrop);
                return Ok(path.file_name().map(|f| f.to_string_lossy().to_string()));
            }
        }
        Ok(None)
    }

    async fn generate_png(&self) -> NativeExtensionsResult<Vec<u8>> {
        let formats = self.data_object_formats()?;
        // prefer DIBV5 with alpha channel
        let data = if formats.contains(&(CF_DIBV5.0 as u32)) {
            Ok(self.data_object.get_data(CF_DIBV5.0 as u32)?)
        } else if formats.contains(&(CF_DIB.0 as u32)) {
            Ok(self.data_object.get_data(CF_DIB.0 as u32)?)
        } else {
            Err(NativeExtensionsError::OtherError(
                "No DIB or DIBV5 data found in data object".into(),
            ))
        }?;
        let mut bmp = Vec::<u8>::new();
        bmp.extend_from_slice(&[0x42, 0x4D]); // BM
        bmp.extend_from_slice(&((data.len() + 14) as u32).to_le_bytes()); // File size
        bmp.extend_from_slice(&[0, 0]); // reserved 1
        bmp.extend_from_slice(&[0, 0]); // reserved 2
        bmp.extend_from_slice(&[0, 0, 0, 0]); // data starting address; not required by decoder
        bmp.extend_from_slice(&data);

        let (future, completer) = FutureCompleter::new();

        let mut completer = Capsule::new(completer);
        let sender = RunLoop::current().new_sender();

        // Do the actual encoding on worker thread
        thread::spawn(move || {
            let stream = unsafe { SHCreateMemStream(Some(&bmp)) };
            let stream = stream.unwrap();
            let res = convert_to_png(stream).map_err(NativeExtensionsError::from);
            sender.send(move || {
                let completer = completer.take().unwrap();
                completer.complete(res);
            });
        });

        future.await
    }

    pub async fn get_data_for_item(
        &self,
        item: i64,
        data_type: String,
        _progress: Option<Arc<ReadProgress>>,
    ) -> NativeExtensionsResult<Value> {
        let format = format_from_string(&data_type);
        let png = unsafe { RegisterClipboardFormatW(w!("PNG")) };
        if format == CF_HDROP.0 as u32 {
            let item = item as usize;
            let hdrop = self.get_hdrop()?.unwrap_or_default();
            if item < hdrop.len() {
                Ok(hdrop[item].clone().into())
            } else {
                Ok(Value::Null)
            }
        } else if format == png && self.need_to_synthetize_png()? {
            let png_data = self.generate_png().await?;
            Ok(png_data.into())
        } else {
            let formats = self.data_object_formats()?;
            if formats.contains(&format) {
                let data = self.data_object.get_data(format)?;
                Ok(data.into())
            } else {
                // possibly virtual
                Ok(Value::Null)
            }
        }
    }

    pub fn new_with_data_object(
        data_object: IDataObject,
        drop_notifier: Option<Arc<DropNotifier>>,
    ) -> Rc<Self> {
        let res = Rc::new(PlatformDataReader {
            data_object,
            _drop_notifier: drop_notifier,
        });
        res.assign_weak_self(Rc::downgrade(&res));
        res
    }

    pub fn new_clipboard_reader() -> NativeExtensionsResult<Rc<Self>> {
        let data_object = unsafe { OleGetClipboard() }?;
        Ok(Self::new_with_data_object(data_object, None))
    }

    pub fn assign_weak_self(&self, _weak: Weak<PlatformDataReader>) {}

    /// Returns parsed hdrop content
    fn get_hdrop(&self) -> NativeExtensionsResult<Option<Vec<String>>> {
        if self.data_object.has_data(CF_HDROP.0 as u32) {
            let data = self.data_object.get_data(CF_HDROP.0 as u32)?;
            Ok(Some(Self::extract_drop_files(data)?))
        } else {
            Ok(None)
        }
    }

    fn get_file_descriptors(&self) -> NativeExtensionsResult<Option<Vec<FileDescriptor>>> {
        let format = unsafe { RegisterClipboardFormatW(CFSTR_FILEDESCRIPTOR) };
        if self.data_object.has_data(format) {
            let data = self.data_object.get_data(format)?;
            Ok(Some(Self::extract_file_descriptors(data)?))
        } else {
            Ok(None)
        }
    }

    fn extract_file_descriptors(buffer: Vec<u8>) -> NativeExtensionsResult<Vec<FileDescriptor>> {
        if buffer.len() < std::mem::size_of::<FILEGROUPDESCRIPTORW>() {
            return Err(NativeExtensionsError::InvalidData);
        }

        let group_descriptor: &FILEGROUPDESCRIPTORW =
            unsafe { &*(buffer.as_ptr() as *const FILEGROUPDESCRIPTORW) };

        if group_descriptor.cItems == 0 {
            return Ok(Vec::new());
        }

        if buffer.len()
            < std::mem::size_of::<FILEGROUPDESCRIPTORW>()
                + (group_descriptor.cItems - 1) as usize * std::mem::size_of::<FILEDESCRIPTORW>()
        {
            return Err(NativeExtensionsError::InvalidData);
        }

        let files = unsafe {
            std::slice::from_raw_parts(
                group_descriptor.fgd.as_ptr(),
                group_descriptor.cItems as usize,
            )
        };

        let res: Vec<_> = files
            .iter()
            .map(|f| {
                let file_name = f.cFileName;
                let len = file_name
                    .iter()
                    .position(|a| *a == 0)
                    .unwrap_or(file_name.len());
                let name = String::from_utf16_lossy(&file_name[0..len]);
                let format = mime_from_name(&name);
                let format = mime_to_windows(format);
                FileDescriptor { name, format }
            })
            .collect();
        Ok(res)
    }

    fn extract_drop_files(buffer: Vec<u8>) -> NativeExtensionsResult<Vec<String>> {
        if buffer.len() < std::mem::size_of::<DROPFILES>() {
            return Err(NativeExtensionsError::InvalidData);
        }
        let files: &DROPFILES = unsafe { &*(buffer.as_ptr() as *const DROPFILES) };

        let mut res = Vec::new();
        if { files.fWide }.as_bool() {
            let data = buffer
                .as_slice()
                .get(files.pFiles as usize..)
                .ok_or(NativeExtensionsError::InvalidData)?
                .as_slice_of::<u16>()
                .map_err(|_| NativeExtensionsError::InvalidData)?;
            let mut offset = 0;
            loop {
                let len = data
                    .get(offset..)
                    .ok_or(NativeExtensionsError::InvalidData)?
                    .iter()
                    .position(|a| *a == 0)
                    .unwrap_or(0);
                if len == 0 {
                    break;
                } else {
                    res.push(String::from_utf16_lossy(
                        data.get(offset..offset + len)
                            .ok_or(NativeExtensionsError::InvalidData)?,
                    ));
                }
                offset += len + 1;
            }
        } else {
            let data = &buffer
                .as_slice()
                .get(files.pFiles as usize..)
                .ok_or(NativeExtensionsError::InvalidData)?;
            let mut offset = 0;
            loop {
                let str = CStr::from_bytes_with_nul(
                    data.get(offset..)
                        .ok_or(NativeExtensionsError::InvalidData)?,
                )
                .unwrap();
                let bytes = str.to_bytes();
                if bytes.is_empty() {
                    break;
                }
                res.push(str.to_string_lossy().into());
                offset += bytes.len();
            }
        }
        Ok(res)
    }

    fn stream_from_medium(medium: &STGMEDIUM) -> NativeExtensionsResult<IStream> {
        match medium.tymed {
            TYMED_HGLOBAL => {
                let stream = unsafe {
                    let size = GlobalSize(medium.Anonymous.hGlobal);
                    let data = GlobalLock(medium.Anonymous.hGlobal);
                    let data = slice::from_raw_parts(data as *const u8, size);
                    let res = SHCreateMemStream(Some(data));
                    GlobalUnlock(medium.Anonymous.hGlobal);
                    res
                };
                match stream {
                    Some(stream) => Ok(stream),
                    None => Err(NativeExtensionsError::VirtualFileReceiveError(
                        "Could not create stream from HGlobal".into(),
                    )),
                }
            }
            TYMED_ISTREAM => match unsafe { medium.Anonymous.pstm.as_ref() } {
                Some(stream) => Ok(stream.clone()),
                None => Err(NativeExtensionsError::VirtualFileReceiveError(
                    "IStream missing".into(),
                )),
            },
            _ => Err(NativeExtensionsError::VirtualFileReceiveError(
                "unsupported data format (unexpected tymed)".into(),
            )),
        }
    }

    pub async fn create_virtual_file_reader_for_item(
        &self,
        item: i64,
        _format: &str,
        _progress: Arc<ReadProgress>,
    ) -> NativeExtensionsResult<Option<Rc<dyn VirtualFileReader>>> {
        let descriptor = self.descriptor_for_virtual_file(item)?;
        let mut medium = self.medium_for_virtual_file(item)?;
        let stream = Self::stream_from_medium(&medium);
        unsafe { ReleaseStgMedium(&mut medium as *mut STGMEDIUM) };
        let stream = stream?;
        let stream = unsafe { Movable::new(stream) };
        let reader = StreamReader::new(stream, descriptor.name).await?;
        Ok(Some(Rc::new(reader)))
    }

    fn do_copy_virtual_file(
        medium: &STGMEDIUM,
        file_name: &str,
        target_folder: PathBuf,
        progress: Arc<ReadProgress>,
        completer: FutureCompleter<NativeExtensionsResult<PathBuf>>,
    ) {
        match medium.tymed {
            TYMED_HGLOBAL => {
                let path = get_target_path(&target_folder, file_name);
                let res = unsafe {
                    let size = GlobalSize(medium.Anonymous.hGlobal);
                    let data = GlobalLock(medium.Anonymous.hGlobal);
                    let data = slice::from_raw_parts(data as *const u8, size);
                    let res = fs::write(&path, data);
                    GlobalUnlock(medium.Anonymous.hGlobal);
                    res
                };
                match res {
                    Ok(_) => completer.complete(Ok(path)),
                    Err(err) => completer.complete(Err(
                        NativeExtensionsError::VirtualFileReceiveError(err.to_string()),
                    )),
                }
            }
            TYMED_ISTREAM => match unsafe { medium.Anonymous.pstm.as_ref() } {
                Some(stream) => {
                    let reader = VirtualStreamCopier {
                        sender: RunLoop::current().new_sender(),
                        stream: unsafe { Movable::new(stream.clone()) },
                        file_name: file_name.into(),
                        target_folder,
                        progress,
                        completer: Capsule::new(completer),
                    };
                    thread::spawn(move || {
                        reader.read();
                    });
                }
                None => completer.complete(Err(NativeExtensionsError::VirtualFileReceiveError(
                    "IStream missing".into(),
                ))),
            },
            _ => completer.complete(Err(NativeExtensionsError::VirtualFileReceiveError(
                "unsupported data format (unexpected tymed)".into(),
            ))),
        }
    }

    fn descriptor_for_virtual_file(&self, item: i64) -> NativeExtensionsResult<FileDescriptor> {
        let descriptors = self.get_file_descriptors()?.ok_or_else(|| {
            NativeExtensionsError::VirtualFileReceiveError(
                "DataObject has not virtual files".into(),
            )
        })?;
        let descriptor = descriptors.get(item as usize).ok_or_else(|| {
            NativeExtensionsError::VirtualFileReceiveError("item not found".into())
        })?;
        Ok(descriptor.clone())
    }

    fn medium_for_virtual_file(&self, item: i64) -> NativeExtensionsResult<STGMEDIUM> {
        let format = unsafe { RegisterClipboardFormatW(CFSTR_FILECONTENTS) };
        let format = make_format_with_tymed_index(
            format,
            TYMED(TYMED_ISTREAM.0 | TYMED_HGLOBAL.0),
            item as i32,
        );
        if self.data_object.has_data_for_format(&format) {
            unsafe {
                let medium = DataObject::with_local_request(|| {
                    self.data_object.GetData(&format as *const _)
                })?;
                Ok(medium)
            }
        } else {
            Err(NativeExtensionsError::VirtualFileReceiveError(
                "item not found".into(),
            ))
        }
    }

    pub async fn copy_virtual_file_for_item(
        &self,
        item: i64,
        _format: &str,
        target_folder: PathBuf,
        progress: Arc<ReadProgress>,
    ) -> NativeExtensionsResult<PathBuf> {
        let descriptor = self.descriptor_for_virtual_file(item)?;
        let mut medium = self.medium_for_virtual_file(item)?;
        unsafe {
            let (future, completer) = FutureCompleter::new();
            Self::do_copy_virtual_file(
                &medium,
                &descriptor.name,
                target_folder,
                progress,
                completer,
            );
            ReleaseStgMedium(&mut medium as *mut STGMEDIUM);
            return future.await;
        }
    }
}

struct StreamReader {
    stream: Movable<IStream>,
    length: u64,
    file_name: String,
    // Single thread thread-pool so that all requests are run in background
    // but serialized.
    thread_pool: ThreadPool,
    read_state: Arc<Mutex<Option<ReadState>>>,
}

struct ReadState {
    num_read: u64,
}

impl StreamReader {
    async fn new(stream: Movable<IStream>, file_name: String) -> NativeExtensionsResult<Self> {
        let thread_pool = ThreadPool::new(1);
        let length = Self::stream_length(&stream, &thread_pool).await?;
        Ok(StreamReader {
            stream,
            length,
            file_name,
            thread_pool,
            read_state: Arc::new(Mutex::new(None)),
        })
    }

    async fn stream_length(
        stream: &Movable<IStream>,
        thread_pool: &ThreadPool,
    ) -> NativeExtensionsResult<u64> {
        fn stream_length(stream: &IStream) -> NativeExtensionsResult<u64> {
            let mut stat = STATSTG::default();
            unsafe {
                stream.Stat(&mut stat as *mut _, STATFLAG_NONAME)?;
            }
            Ok(stat.cbSize)
        }

        let (future, completer) = FutureCompleter::new();
        let stream_clone = stream.clone();
        let sender = RunLoop::current().new_sender();
        let mut completer = Capsule::new_with_sender(completer, sender.clone());
        thread_pool.execute(move || {
            let len = stream_length(&stream_clone);
            sender.send(move || {
                completer.take().unwrap().complete(len);
            });
        });

        future.await
    }

    fn read(
        stream: Movable<IStream>,
        length: u64,
        state: Arc<Mutex<Option<ReadState>>>,
    ) -> NativeExtensionsResult<Vec<u8>> {
        let mut state = state.lock().unwrap();
        if state.is_none() {
            state.replace(ReadState { num_read: 0 });
            unsafe {
                stream.Seek(0, STREAM_SEEK_SET, None)?;
            }
        }
        let mut buf = Vec::<u8>::new();
        buf.resize(1024 * 1024, 0);
        let mut state = state.as_mut().unwrap();
        let to_read = (length - state.num_read).min(buf.len() as u64) as u32;
        if to_read == 0 {
            Ok(Vec::new())
        } else {
            let mut did_read = 0u32;
            let res = unsafe {
                stream.Read(
                    buf.as_ptr() as *mut _,
                    to_read,
                    Some(&mut did_read as *mut _),
                )
            };
            if res != S_OK {
                Err(windows::core::Error::from(res).into())
            } else {
                state.num_read += did_read as u64;
                buf.resize(did_read as usize, 0);
                Ok(buf)
            }
        }
    }
}

#[async_trait(?Send)]
impl VirtualFileReader for StreamReader {
    async fn read_next(&self) -> NativeExtensionsResult<Vec<u8>> {
        let (future, completer) = FutureCompleter::new();
        let sender = RunLoop::current().new_sender();
        let mut completer = Capsule::new_with_sender(completer, sender.clone());
        let stream = self.stream.clone();
        let read_state = self.read_state.clone();
        let length = self.length;
        self.thread_pool.execute(move || {
            let res = Self::read(stream, length, read_state);
            sender.send(move || {
                completer.take().unwrap().complete(res);
            });
        });
        future.await
    }

    fn file_size(&self) -> NativeExtensionsResult<Option<i64>> {
        Ok(Some(self.length as i64))
    }

    fn file_name(&self) -> Option<String> {
        Some(self.file_name.clone())
    }

    fn close(&self) -> NativeExtensionsResult<()> {
        // Stream gets closed upon release
        Ok(())
    }
}

// Most streams in COM should be agile, also the documentation for IDataObjectAsyncCapability
// assumes that the stream is read on background thread so we wrap it inside Movable
// in order to be able to send it.
struct VirtualStreamCopier {
    sender: RunLoopSender,
    stream: Movable<IStream>,
    file_name: String,
    target_folder: PathBuf,
    progress: Arc<ReadProgress>,
    completer: Capsule<FutureCompleter<NativeExtensionsResult<PathBuf>>>,
}

impl VirtualStreamCopier {
    fn get_length(&self) -> NativeExtensionsResult<u64> {
        let mut stat = STATSTG::default();
        unsafe {
            self.stream.Stat(&mut stat as *mut _, STATFLAG_NONAME)?;
        }
        Ok(stat.cbSize)
    }

    fn read_inner(&self) -> NativeExtensionsResult<PathBuf> {
        let temp_name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();
        let temp_path = self.target_folder.join(format!(".{temp_name}"));
        let file = File::create(&temp_path)?;
        unsafe {
            let path: String = temp_path.to_string_lossy().into();
            let path = HSTRING::from(path);
            SetFileAttributesW(&path, FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_TEMPORARY);
        }
        match self.read_and_write(file) {
            Ok(_) => {
                let path = get_target_path(&self.target_folder, &self.file_name);
                fs::rename(temp_path, &path)?;
                unsafe {
                    let path: String = path.to_string_lossy().into();
                    let path = HSTRING::from(path);
                    SetFileAttributesW(&path, FILE_ATTRIBUTE_ARCHIVE);
                }
                Ok(path)
            }
            Err(err) => {
                fs::remove_file(temp_path).ok_log();
                Err(err)
            }
        }
    }

    fn read_and_write(&self, mut f: File) -> NativeExtensionsResult<()> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancelled_clone = cancelled.clone();
        self.progress
            .set_cancellation_handler(Some(Box::new(move || {
                cancelled_clone.store(true, Ordering::Release);
            })));
        let length = self.get_length()?;
        let mut num_read: u64 = 0;
        let mut buf = Vec::<u8>::new();
        buf.resize(1024 * 1024, 0);
        let mut last_reported_progress = 0f64;

        unsafe {
            self.stream.Seek(0, STREAM_SEEK_SET, None)?;
        }

        loop {
            if cancelled.load(Ordering::Acquire) {
                return Err(NativeExtensionsError::VirtualFileReceiveError(
                    "cancelled".into(),
                ));
            }
            let to_read = (length - num_read).min(buf.len() as u64) as u32;
            if to_read == 0 {
                break;
            }
            let mut did_read = 0u32;
            let res = unsafe {
                self.stream.Read(
                    buf.as_ptr() as *mut _,
                    to_read,
                    Some(&mut did_read as *mut _),
                )
            };
            if res != S_OK {
                return Err(windows::core::Error::from(res).into());
            }
            if did_read == 0 {
                return Err(NativeExtensionsError::VirtualFileReceiveError(
                    "stream ended prematurely".into(),
                ));
            }
            f.write_all(&buf[..did_read as usize])?;
            num_read += did_read as u64;

            let progress = num_read as f64 / length as f64;
            if progress >= last_reported_progress + 0.05 {
                last_reported_progress = progress;
                self.progress.report_progress(Some(progress));
            }
        }
        self.progress.report_progress(Some(1.0));

        Ok(())
    }

    fn read(self) {
        let res = self.read_inner();
        let mut completer = self.completer;
        self.sender.send(move || {
            let completer = completer.take().unwrap();
            completer.complete(res);
        });
    }
}

// Map mime types to known windows clipboard format
fn mime_to_windows(fmt: String) -> String {
    match fmt.as_str() {
        "image/png" => "PNG".to_owned(),
        "image/jpeg" => "JFIF".to_string(),
        "image/gif" => "GIF".to_string(),
        "image/tiff" => format_to_string(CF_TIFF.0 as u32),
        _ => fmt,
    }
}

fn mime_from_name(name: &str) -> String {
    let ext = Path::new(name).extension();
    mime_guess::from_path(name)
        .first()
        .map(|m| m.to_string())
        .unwrap_or_else(|| {
            format!(
                "application/octet-stream;extension={}",
                ext.unwrap_or_default().to_string_lossy()
            )
        })
}
