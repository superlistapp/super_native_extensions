use nativeshell_core::{IntoValue, TryFromValue, Value};

#[derive(Clone, Debug, Default, PartialEq, TryFromValue, IntoValue)]
#[nativeshell(rename_all = "camelCase")]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Default, PartialEq, TryFromValue, IntoValue)]
#[nativeshell(rename_all = "camelCase")]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Default, PartialEq, TryFromValue, IntoValue)]
#[nativeshell(rename_all = "camelCase")]
pub struct ImageData {
    pub width: i32,
    pub height: i32,
    pub bytes_per_row: i32,
    /// Pixel data as RGBA bytes.
    pub data: Vec<u8>,
    pub device_pixel_ratio: Option<f64>,
}

//
// Clipboard Writer
//

#[derive(Debug, TryFromValue, IntoValue, Clone, Copy, PartialEq, Hash, Eq)]
pub struct PlatformWriterId(i64);

impl From<i64> for PlatformWriterId {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

#[derive(Debug, TryFromValue, IntoValue, Clone, Copy, PartialEq, Hash, Eq)]
pub struct LazyValueId(i64);

#[derive(Debug, TryFromValue, IntoValue, Clone, PartialEq)]
#[nativeshell(tag = "type", rename_all = "camelCase")]
pub enum ClipboardWriterItemData {
    #[nativeshell(rename_all = "camelCase")]
    Simple { types: Vec<String>, data: Value },
    #[nativeshell(rename_all = "camelCase")]
    Lazy { types: Vec<String>, id: LazyValueId },
    #[nativeshell(rename_all = "camelCase")]
    VirtualFile { file_size: i64, file_name: String },
}

#[derive(Debug, TryFromValue, IntoValue, Clone, PartialEq)]
#[nativeshell(rename_all = "camelCase")]
pub struct ClipboardWriterItem {
    pub data: Vec<ClipboardWriterItemData>,
}

#[derive(Debug, TryFromValue, IntoValue, Clone, PartialEq)]
#[nativeshell(rename_all = "camelCase")]
pub struct ClipboardWriterData {
    pub items: Vec<ClipboardWriterItem>,
}

//

#[derive(Debug, TryFromValue, IntoValue, Clone, Copy, PartialEq, Hash, Eq)]
pub struct DataSourceId(i64);

impl From<i64> for DataSourceId {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

#[derive(Debug, TryFromValue, IntoValue, Clone, PartialEq)]
#[nativeshell(tag = "type", rename_all = "camelCase")]
pub enum DataSourceItemRepresentation {
    #[nativeshell(rename_all = "camelCase")]
    Simple { formats: Vec<String>, data: Value },
    #[nativeshell(rename_all = "camelCase")]
    Lazy {
        formats: Vec<String>,
        id: LazyValueId,
    },
    #[nativeshell(rename_all = "camelCase")]
    VirtualFile { file_size: i64, file_name: String },
}

#[derive(Debug, TryFromValue, IntoValue, Clone, PartialEq)]
#[nativeshell(rename_all = "camelCase")]
pub struct DataSourceItem {
    pub representations: Vec<DataSourceItemRepresentation>,
}

#[derive(Debug, TryFromValue, IntoValue, Clone, PartialEq)]
#[nativeshell(rename_all = "camelCase")]
pub struct DataSource {
    pub items: Vec<DataSourceItem>,
}
