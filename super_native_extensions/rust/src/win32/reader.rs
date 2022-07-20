use std::{
    ffi::CStr,
    rc::{Rc, Weak},
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

use crate::error::NativeExtensionsResult;

use super::common::{extract_formats, format_from_string, format_to_string, get_data, has_data};

pub struct PlatformDataReader {
    data_object: IDataObject,
}

impl PlatformDataReader {
    pub async fn get_items(&self) -> NativeExtensionsResult<Vec<i64>> {
        Ok((0..self.item_count()? as i64).collect())
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

    pub async fn get_formats_for_item(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
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

    fn new_with_data_object(data_object: IDataObject) -> NativeExtensionsResult<Rc<Self>> {
        let res = Rc::new(PlatformDataReader { data_object });
        res.assign_weak_self(Rc::downgrade(&res));
        Ok(res)
    }

    pub fn new_clipboard_reader() -> NativeExtensionsResult<Rc<Self>> {
        let data_object = unsafe { OleGetClipboard() }?;
        Self::new_with_data_object(data_object)
    }

    pub fn assign_weak_self(&self, _weak: Weak<PlatformDataReader>) {}

    fn get_hdrop(&self) -> NativeExtensionsResult<Option<Vec<String>>> {
        if has_data(&self.data_object, CF_HDROP.0 as u32) {
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
