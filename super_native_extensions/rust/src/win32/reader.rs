use std::{
    ffi::CStr,
    path::{Path, PathBuf},
    rc::{Rc, Weak},
    sync::Arc,
};

use byte_slice_cast::AsSliceOf;
use nativeshell_core::Value;
use windows::Win32::{
    System::{
        Com::{IDataObject, TYMED_HGLOBAL},
        DataExchange::RegisterClipboardFormatW,
        Ole::OleGetClipboard,
        SystemServices::CF_HDROP,
    },
    UI::Shell::{CFSTR_FILEDESCRIPTOR, DROPFILES, FILEDESCRIPTORW, FILEGROUPDESCRIPTORW},
};

use crate::{
    error::{NativeExtensionsError, NativeExtensionsResult},
    reader_manager::ReadProgress,
    util::DropNotifier,
};

use super::{
    common::{extract_formats, format_from_string, format_to_string},
    data_object::GetData,
};

pub struct PlatformDataReader {
    data_object: IDataObject,
    _drop_notifier: Option<Arc<DropNotifier>>,
}

/// Virtual file descriptor
struct FileDescriptor {
    index: usize,
    name: String,
    format: String,
}

impl PlatformDataReader {
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
        } else if !self.supported_formats()?.is_empty() {
            Ok(1)
        } else {
            Ok(0)
        }
    }

    fn supported_formats(&self) -> NativeExtensionsResult<Vec<u32>> {
        let formats = extract_formats(&self.data_object)?
            .iter()
            .filter_map(|f| {
                if f.tymed == TYMED_HGLOBAL.0 as u32 {
                    Some(f.cfFormat as u32)
                } else {
                    None
                }
            })
            .collect();
        Ok(formats)
    }

    pub fn get_formats_for_item_sync(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        let mut formats = if item == 0 {
            self.supported_formats()?
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

    pub async fn get_data_for_item(
        &self,
        item: i64,
        data_type: String,
        _progress: Arc<ReadProgress>,
    ) -> NativeExtensionsResult<Value> {
        let format = format_from_string(&data_type);
        if format == CF_HDROP.0 as u32 {
            let item = item as usize;
            let hdrop = self.get_hdrop()?.unwrap_or_default();
            if item < hdrop.len() {
                Ok(hdrop[item].clone().into())
            } else {
                Ok(Value::Null)
            }
        } else {
            let data = self.data_object.get_data(format)?;
            Ok(data.into())
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
            .enumerate()
            .map(|(index, f)| {
                let file_name = f.cFileName;
                let len = file_name
                    .iter()
                    .position(|a| *a == 0)
                    .unwrap_or(file_name.len());
                let name = String::from_utf16_lossy(&file_name[0..len]);
                let ext = Path::new(&name).extension();
                let format = mime_guess::from_path(&name)
                    .first()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| {
                        format!(
                            "application/octet-stream;extension={}",
                            ext.unwrap_or_default().to_string_lossy()
                        )
                    });
                FileDescriptor {
                    index,
                    name,
                    format,
                }
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
                        &data
                            .get(offset..offset + len)
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
                    &data
                        .get(offset..)
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

    pub async fn can_get_virtual_file_for_item(
        &self,
        item: i64,
        format: &str,
    ) -> NativeExtensionsResult<bool> {
        let descriptors = self.get_file_descriptors()?;
        if let Some(descriptors) = descriptors {
            if let Some(descriptor) = descriptors.get(item as usize) {
                return Ok(format == descriptor.format);
            }
        }

        Ok(false)
    }

    pub async fn get_virtual_file_for_item(
        &self,
        item: i64,
        _format: &str,
        target_folder: PathBuf,
        _progress: Arc<ReadProgress>,
    ) -> NativeExtensionsResult<PathBuf> {
        let descriptors = self.get_file_descriptors()?.ok_or_else(|| {
            NativeExtensionsError::VirtualFileReceiveError(
                "DataObject has not virtual files".into(),
            )
        })?;
        let descriptor = descriptors.get(item as usize).ok_or_else(|| {
            NativeExtensionsError::VirtualFileReceiveError("item not found".into())
        })?;
        println!("FN {:?}", descriptor.name);
        Err(NativeExtensionsError::UnsupportedOperation)
    }
}
