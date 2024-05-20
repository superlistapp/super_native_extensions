use std::{fs::OpenOptions, io::Write, mem::size_of, path::Path, ptr::null_mut};

use once_cell::sync::Lazy;
use windows::{
    core::{s, ComInterface, GUID, HRESULT, HSTRING},
    Win32::{
        Foundation::{E_UNEXPECTED, HANDLE, HWND, S_OK},
        Graphics::Gdi::{
            CreateDIBSection, GetDC, GetDeviceCaps, MonitorFromWindow, ReleaseDC, BITMAPINFO,
            BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HMONITOR, LOGPIXELSX,
            MONITOR_DEFAULTTOPRIMARY,
        },
        System::{
            Com::{
                CoCreateInstance, IDataObject, IStream, CLSCTX_ALL, DATADIR_GET, DVASPECT_CONTENT,
                FORMATETC, TYMED,
            },
            DataExchange::{GetClipboardFormatNameW, RegisterClipboardFormatW},
            LibraryLoader::{GetProcAddress, LoadLibraryA},
        },
        UI::HiDpi::{MDT_EFFECTIVE_DPI, MONITOR_DPI_TYPE},
    },
};

use crate::{
    api_model::ImageData,
    error::{NativeExtensionsError, NativeExtensionsResult},
};

const INTERNAL_PREFIX: &str = "NativeShell_CF_";

pub fn format_to_string(format: u32) -> String {
    let mut buf: [_; 1024] = [0u16; 1024];
    let len = unsafe { GetClipboardFormatNameW(format, &mut buf) };
    if len == 0 {
        format!("{INTERNAL_PREFIX}{format}")
    } else {
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

pub fn format_from_string(format: &str) -> u32 {
    if let Some(format) = format.strip_prefix(INTERNAL_PREFIX) {
        format.parse::<u32>().ok().unwrap_or(0)
    } else {
        unsafe { RegisterClipboardFormatW(&HSTRING::from(format)) }
    }
}

pub fn make_format_with_tymed(format: u32, tymed: TYMED) -> FORMATETC {
    make_format_with_tymed_index(format, tymed, -1)
}

pub fn make_format_with_tymed_index(format: u32, tymed: TYMED, index: i32) -> FORMATETC {
    FORMATETC {
        cfFormat: format as u16,
        ptd: null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: index,
        tymed: tymed.0 as u32,
    }
}

impl From<windows::core::Error> for NativeExtensionsError {
    fn from(error: windows::core::Error) -> Self {
        NativeExtensionsError::OtherError(format!("Windows Error: {error}"))
    }
}

/// # Safety
///
/// Data must be properly aligned (see slice::from_raw_parts)
pub unsafe fn as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>())
}

pub fn extract_formats(object: &IDataObject) -> windows::core::Result<Vec<FORMATETC>> {
    let e = unsafe { object.EnumFormatEtc(DATADIR_GET.0 as u32)? };
    let mut res = Vec::new();
    loop {
        let mut format = [FORMATETC::default()];
        let mut fetched = 0u32;
        if unsafe { e.Next(&mut format, Some(&mut fetched as *mut _)) }.is_err() || fetched == 0 {
            break;
        }
        res.push(format[0]);
    }
    Ok(res)
}

pub fn image_data_to_hbitmap(image: &ImageData) -> NativeExtensionsResult<HBITMAP> {
    let bitmap = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: image.width,
            biHeight: image.height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            biSizeImage: (image.width * image.height * 4) as u32,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: Default::default(),
    };

    unsafe {
        let dc = GetDC(HWND(0));

        let mut ptr = std::ptr::null_mut();

        let bitmap = CreateDIBSection(
            dc,
            &bitmap as *const _,
            DIB_RGB_COLORS,
            &mut ptr as *mut *mut _,
            HANDLE(0),
            0,
        )?;

        // Bitmap needs to be flipped and unpremultiplied

        let dst_stride = (image.width * 4) as isize;
        let ptr = ptr as *mut u8;
        for y in 0..image.height as isize {
            let src_line = image
                .data
                .as_ptr()
                .offset((image.height as isize - y - 1) * image.bytes_per_row as isize);

            let dst_line = ptr.offset(y * dst_stride);

            for x in (0..dst_stride).step_by(4) {
                let (r, g, b, a) = (
                    *src_line.offset(x) as i32,
                    *src_line.offset(x + 1) as i32,
                    *src_line.offset(x + 2) as i32,
                    *src_line.offset(x + 3) as i32,
                );

                // ByteFormat.rawStraightRgba already has unpremultiplied alpha
                // but channel order is different.

                *dst_line.offset(x) = b as u8;
                *dst_line.offset(x + 1) = g as u8;
                *dst_line.offset(x + 2) = r as u8;
                *dst_line.offset(x + 3) = a as u8;
            }
        }

        ReleaseDC(HWND(0), dc);

        Ok(bitmap)
    }
}

pub fn create_instance<T: ComInterface>(clsid: &GUID) -> windows::core::Result<T> {
    unsafe { CoCreateInstance(clsid, None, CLSCTX_ALL) }
}

impl From<NativeExtensionsError> for windows::core::Error {
    fn from(err: NativeExtensionsError) -> Self {
        windows::core::Error::new(E_UNEXPECTED, err.to_string().into())
    }
}

type GetDpiForMonitor = unsafe extern "system" fn(
    hmonitor: HMONITOR,
    dpitype: MONITOR_DPI_TYPE,
    dpix: *mut u32,
    dpiy: *mut u32,
) -> HRESULT;

type GetDpiForWindow = unsafe extern "system" fn(hwnd: HWND) -> u32;

struct DpiFunctions {
    get_dpi_for_window: Option<GetDpiForWindow>,
    get_dpi_for_monitor: Option<GetDpiForMonitor>,
}

impl DpiFunctions {
    fn new() -> Self {
        unsafe {
            let user_32 = LoadLibraryA(s!("user32")).unwrap();
            let shlib = LoadLibraryA(s!("Shcore.dll")).unwrap();
            Self {
                #[allow(clippy::missing_transmute_annotations)]
                get_dpi_for_window: std::mem::transmute(GetProcAddress(
                    user_32,
                    s!("GetDpiForWindow"),
                )),
                #[allow(clippy::missing_transmute_annotations)]
                get_dpi_for_monitor: std::mem::transmute(GetProcAddress(
                    shlib,
                    s!("GetDpiForMonitor"),
                )),
            }
        }
    }
}

static DPI_FUNCTIONS: Lazy<DpiFunctions> = Lazy::new(DpiFunctions::new);

pub fn get_dpi_for_window(hwnd: HWND) -> u32 {
    if let Some(get_dpi_for_window) = DPI_FUNCTIONS.get_dpi_for_window {
        return unsafe { get_dpi_for_window(hwnd) };
    }
    if let Some(get_dpi_for_monitor) = DPI_FUNCTIONS.get_dpi_for_monitor {
        let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY) };
        let mut dpi_x = 0u32;
        let mut dpi_y = 0u32;
        if unsafe {
            get_dpi_for_monitor(
                monitor,
                MDT_EFFECTIVE_DPI,
                &mut dpi_x as *mut _,
                &mut dpi_y as *mut _,
            )
        } == S_OK
        {
            return dpi_x;
        }
    }
    unsafe {
        let hdc = GetDC(hwnd);
        let dpi = GetDeviceCaps(hdc, LOGPIXELSX);
        ReleaseDC(hwnd, hdc);
        dpi as u32
    }
}

fn read_stream_fully_with<F: FnMut(&[u8]) -> bool>(
    stream: &IStream,
    mut fun: F,
) -> windows::core::Result<()> {
    let mut buf: [u8; 256 * 1024] = [0; 256 * 1024];
    loop {
        let mut num_read: u32 = 0;
        let res = unsafe {
            stream.Read(
                buf.as_mut_ptr() as *mut _,
                buf.len() as u32,
                Some(&mut num_read as *mut _),
            )
        };
        if res.is_err() {
            return Err(res.into());
        }

        if num_read == 0 {
            break;
        }
        if !fun(&buf[..num_read as usize]) {
            break;
        }
    }
    Ok(())
}

pub fn read_stream_fully(stream: &IStream) -> windows::core::Result<Vec<u8>> {
    let mut res = Vec::<u8>::new();
    read_stream_fully_with(stream, |b| {
        res.extend_from_slice(b);
        true
    })?;
    Ok(res)
}

pub fn copy_stream_to_file(stream: &IStream, path: &Path) -> NativeExtensionsResult<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)?;

    let mut res = Ok(());

    read_stream_fully_with(stream, |b| {
        let write_res = file.write_all(b);
        match write_res {
            Ok(_) => true,
            Err(err) => {
                res = Err(err.into());
                false
            }
        }
    })?;

    res
}
