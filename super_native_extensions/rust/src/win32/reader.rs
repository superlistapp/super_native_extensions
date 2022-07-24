use std::{
    ffi::CStr,
    rc::{Rc, Weak},
    sync::Arc,
};

use byte_slice_cast::AsSliceOf;
use nativeshell_core::Value;
use windows::Win32::{
    System::{
        Com::{IDataObject, TYMED_HGLOBAL},
        Ole::OleGetClipboard,
        SystemServices::CF_HDROP,
    },
    UI::Shell::DROPFILES,
};

use crate::{error::NativeExtensionsResult, util::DropNotifier};

use super::{
    common::{extract_formats, format_from_string, format_to_string, get_data},
    data_object::GetData,
};

pub struct PlatformDataReader {
    data_object: IDataObject,
    _drop_notifier: Option<Arc<DropNotifier>>,
}

impl PlatformDataReader {
    pub fn get_items_sync(&self) -> NativeExtensionsResult<Vec<i64>> {
        Ok((0..self.item_count()? as i64).collect())
    }

    pub async fn get_items(&self) -> NativeExtensionsResult<Vec<i64>> {
        self.get_items_sync()
    }

    fn item_count(&self) -> NativeExtensionsResult<usize> {
        let hdrop_len = self.get_hdrop()?.map(|f| f.len()).unwrap_or(0);
        if hdrop_len > 0 {
            Ok(hdrop_len)
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
        if item == 0 {
            return Ok(self
                .supported_formats()?
                .iter()
                .map(|f| format_to_string(*f))
                .collect());
        } else if item > 0 {
            let hdrop_len = self.get_hdrop()?.map(|v| v.len()).unwrap_or(0);
            if item < hdrop_len as i64 {
                return Ok(vec![format_to_string(CF_HDROP.0 as u32)]);
            }
        }
        Ok(vec![])
    }

    pub async fn get_formats_for_item(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        self.get_formats_for_item_sync(item)
    }

    pub async fn get_data_for_item(
        &self,
        item: i64,
        data_type: String,
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
            let data = get_data(&self.data_object, format)?;
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

    fn get_hdrop(&self) -> NativeExtensionsResult<Option<Vec<String>>> {
        if self.data_object.has_data(CF_HDROP.0 as u32) {
            let data = get_data(&self.data_object, CF_HDROP.0 as u32)?;
            Ok(Some(Self::extract_files(data)))
        } else {
            Ok(None)
        }
    }

    //

    fn extract_files(buffer: Vec<u8>) -> Vec<String> {
        let files: &DROPFILES = unsafe { &*(buffer.as_ptr() as *const DROPFILES) };

        let mut res = Vec::new();
        if { files.fWide }.as_bool() {
            let data = buffer.as_slice()[files.pFiles as usize..]
                .as_slice_of::<u16>()
                .unwrap();
            let mut offset = 0;
            loop {
                let len = data[offset..].iter().position(|a| *a == 0).unwrap_or(0);
                if len == 0 {
                    break;
                } else {
                    res.push(String::from_utf16_lossy(&data[offset..offset + len]));
                }
                offset += len + 1;
            }
        } else {
            let data = &buffer.as_slice()[files.pFiles as usize..];
            let mut offset = 0;
            loop {
                let str = CStr::from_bytes_with_nul(&data[offset..]).unwrap();
                let bytes = str.to_bytes();
                if bytes.is_empty() {
                    break;
                }
                res.push(str.to_string_lossy().into());
                offset += bytes.len();
            }
        }
        res
    }
}
