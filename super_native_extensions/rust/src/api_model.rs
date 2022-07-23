use nativeshell_core::{IntoValue, TryFromValue, Value};

#[derive(Clone, Debug, Default, PartialEq, TryFromValue, IntoValue)]
#[nativeshell(rename_all = "camelCase")]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn center(&self) -> Point {
        Point {
            x: self.x + self.width / 2.0,
            y: self.y + self.height / 2.0,
        }
    }

    pub fn translated(&self, x: f64, y: f64) -> Rect {
        Rect {
            x: self.x + x,
            y: self.y + y,
            width: self.width,
            height: self.height,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, TryFromValue, IntoValue)]
#[nativeshell(rename_all = "camelCase")]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Default, PartialEq, TryFromValue, IntoValue)]
#[nativeshell(rename_all = "camelCase")]
pub struct Size {
    pub width: f64,
    pub height: f64,
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
// Data Provider
//

#[derive(Debug, TryFromValue, IntoValue, Clone, Copy, PartialEq, Hash, Eq)]
pub struct DataProviderValueId(i64);

#[derive(Debug, TryFromValue, IntoValue, Clone, Copy, PartialEq, Hash, Eq)]
pub struct DataProviderId(i64);

impl From<i64> for DataProviderId {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

#[derive(Debug, TryFromValue, IntoValue, Clone, PartialEq)]
#[nativeshell(tag = "type", rename_all = "camelCase")]
pub enum DataRepresentation {
    #[nativeshell(rename_all = "camelCase")]
    Simple { format: String, data: Value },
    #[nativeshell(rename_all = "camelCase")]
    Lazy {
        id: DataProviderValueId,
        format: String,
    },
    #[nativeshell(rename_all = "camelCase")]
    VirtualFile {
        id: DataProviderValueId,
        format: String,
        storage_suggestion: Option<VirtualFileStorage>,
    },
}

impl DataRepresentation {
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
pub struct DataProvider {
    pub representations: Vec<DataRepresentation>,
    pub suggested_name: Option<String>,
}

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

//

#[derive(TryFromValue, Debug)]
#[nativeshell(rename_all = "camelCase")]
pub struct DragImage {
    pub image_data: ImageData,
    pub source_rect: Rect,
}

#[derive(TryFromValue, Debug)]
#[nativeshell(rename_all = "camelCase")]
pub struct DragItem {
    pub data_provider_id: DataProviderId,
    /// optionally used on iPad during lifting (before dragging start)
    pub lift_image: Option<DragImage>,
    pub image: DragImage,
    pub local_data: Value,
}

#[derive(TryFromValue, Debug)]
#[nativeshell(rename_all = "camelCase")]
pub struct DragConfiguration {
    pub items: Vec<DragItem>,
    pub allowed_operations: Vec<DropOperation>,
    pub animates_to_starting_position_on_cancel_or_fail: bool,
    pub prefers_full_size_previews: bool,
}

#[derive(TryFromValue)]
#[nativeshell(rename_all = "camelCase")]
pub struct DragRequest {
    pub configuration: DragConfiguration,
    pub combined_drag_image: Option<DragImage>,
    pub position: Point,
}

#[derive(Debug, TryFromValue, IntoValue, Copy, Clone, PartialEq)]
#[nativeshell(rename_all = "camelCase")]
pub enum DropOperation {
    None,
    UserCancelled, // macOS, windows - drag cancelled by user pressing escape
    Forbidden,     // Used on iOS, maps to None on other platforms
    Copy,          // macOS, iOS, Windows, Linux, Android
    Move,          // macOS, iOS (within same app), Windows, Linux
    Link,          // macOS, Windows, Linux
}
