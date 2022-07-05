use std::{ptr::null_mut, slice};

use windows::Win32::System::{
    Com::{IDataObject, DATADIR_GET, DVASPECT_CONTENT, FORMATETC, STGMEDIUM, TYMED, TYMED_HGLOBAL},
    DataExchange::{GetClipboardFormatNameW, RegisterClipboardFormatW},
    Memory::{GlobalLock, GlobalSize, GlobalUnlock},
    Ole::ReleaseStgMedium,
};

use crate::error::NativeExtensionsError;

const INTERNAL_PREFIX: &str = "NativeShell_InternalWindowsFormat_";

pub fn format_to_string(format: u32) -> String {
    let mut buf: [_; 1024] = [0u16; 1024];
    let len = unsafe { GetClipboardFormatNameW(format, &mut buf) };
    if len == 0 {
        format!("{}{}", INTERNAL_PREFIX, format)
    } else {
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

pub fn format_from_string(format: &str) -> u32 {
    if let Some(format) = format.strip_prefix(INTERNAL_PREFIX) {
        format.parse::<u32>().ok().unwrap_or(0)
    } else {
        unsafe { RegisterClipboardFormatW(format) }
    }
}

pub fn make_format_with_tymed(format: u32, tymed: TYMED) -> FORMATETC {
    FORMATETC {
        cfFormat: format as u16,
        ptd: null_mut(),
        dwAspect: DVASPECT_CONTENT.0 as u32,
        lindex: -1,
        tymed: tymed.0 as u32,
    }
}

impl From<windows::core::Error> for NativeExtensionsError {
    fn from(error: windows::core::Error) -> Self {
        NativeExtensionsError::OtherError(format!("Windows Error: {}", error))
    }
}

/// # Safety
///
/// Data must be properly aligned (see slice::from_raw_parts)
pub unsafe fn as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>())
}

pub fn get_data(object: &IDataObject, format: u32) -> windows::core::Result<Vec<u8>> {
    let mut format = make_format_with_tymed(format, TYMED_HGLOBAL);

    unsafe {
        let mut medium = object.GetData(&mut format as *mut _)?;

        let size = GlobalSize(medium.Anonymous.hGlobal);
        let data = GlobalLock(medium.Anonymous.hGlobal);

        let v = slice::from_raw_parts(data as *const u8, size);
        let res: Vec<u8> = v.into();

        GlobalUnlock(medium.Anonymous.hGlobal);

        ReleaseStgMedium(&mut medium as *mut STGMEDIUM);

        Ok(res)
    }
}

pub fn has_data(object: &IDataObject, format: u32) -> bool {
    let mut format = make_format_with_tymed(format, TYMED_HGLOBAL);
    unsafe { object.QueryGetData(&mut format as *mut _).is_ok() }
}

pub fn extract_formats(object: &IDataObject) -> windows::core::Result<Vec<FORMATETC>> {
    let e = unsafe { object.EnumFormatEtc(DATADIR_GET.0 as u32)? };
    let mut res = Vec::new();
    loop {
        let mut format = [FORMATETC::default()];
        let mut fetched = 0u32;
        if unsafe { e.Next(&mut format, &mut fetched as *mut _) }.is_err() || fetched == 0 {
            break;
        }
        res.push(format[0]);
    }
    Ok(res)
}
