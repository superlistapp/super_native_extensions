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
// Data Source
//

#[derive(Debug, TryFromValue, IntoValue, Clone, Copy, PartialEq, Hash, Eq)]
pub struct DataSourceValueId(i64);

//

#[derive(Debug, TryFromValue, IntoValue, Clone, Copy, PartialEq, Hash, Eq)]
pub struct DataSourceId(i64);

impl From<i64> for DataSourceId {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

#[derive(Debug, TryFromValue, IntoValue, Copy, Clone, PartialEq)]
#[nativeshell(rename_all = "camelCase")]
pub enum VirtualFileStorage {
    TemporaryFile,
    Memory,
}

#[derive(Debug, TryFromValue, IntoValue, Clone, PartialEq)]
#[nativeshell(tag = "type", rename_all = "camelCase")]
pub enum DataSourceItemRepresentation {
    #[nativeshell(rename_all = "camelCase")]
    Simple { formats: Vec<String>, data: Value },
    #[nativeshell(rename_all = "camelCase")]
    Lazy {
        id: DataSourceValueId,
        formats: Vec<String>,
    },
    #[nativeshell(rename_all = "camelCase")]
    VirtualFile {
        id: DataSourceValueId,
        format: String,
        storage_suggestion: Option<VirtualFileStorage>,
    },
}

impl DataSourceItemRepresentation {
    pub fn is_virtual_file(&self) -> bool {
        if let Self::VirtualFile {
            id: _,
            format: _,
            storage_suggestion: _,
        } = self
        {
            true
        } else {
            false
        }
    }
}

#[derive(Debug, TryFromValue, IntoValue, Clone, PartialEq)]
#[nativeshell(rename_all = "camelCase")]
pub struct DataSourceItem {
    pub representations: Vec<DataSourceItemRepresentation>,
    pub suggested_name: Option<String>,
}

#[derive(Debug, TryFromValue, IntoValue, Clone, PartialEq)]
#[nativeshell(rename_all = "camelCase")]
pub struct DataSource {
    pub items: Vec<DataSourceItem>,
}

//

#[derive(TryFromValue, Debug)]
#[nativeshell(rename_all = "camelCase")]
pub struct DragImage {
    pub image_data: ImageData,
    pub point_in_rect: Point,
}

#[derive(TryFromValue, Debug)]
#[nativeshell(rename_all = "camelCase")]
pub struct DragData {
    pub data_source_id: DataSourceId,
    pub allowed_operations: Vec<DropOperation>,
    pub drag_image: DragImage,
}

#[derive(TryFromValue)]
#[nativeshell(rename_all = "camelCase")]
pub struct DragRequest {
    pub drag_data: DragData,
    pub drag_position: Point,
}

#[derive(Debug, TryFromValue, IntoValue, Copy, Clone, PartialEq)]
#[nativeshell(rename_all = "camelCase")]
pub enum DropOperation {
    None,
    Forbidden, // Used on iOS, maps to None on other platforms
    Copy,      // macOS, iOS, Windows, Linux, Android
    Move,      // macOS, iOS (within same app), Windows, Linux
    Link,      // macOS, Windows, Linux
}
