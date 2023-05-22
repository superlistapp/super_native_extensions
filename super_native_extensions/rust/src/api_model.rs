use std::rc::Rc;

use irondash_message_channel::{IntoValue, TryFromValue, Value};

use crate::platform_impl::platform::PlatformMenu;

#[derive(Clone, Debug, Default, PartialEq, TryFromValue, IntoValue)]
#[irondash(rename_all = "camelCase")]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn xywh(x: f64, y: f64, width: f64, height: f64) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    pub fn center(&self) -> Point {
        Point {
            x: self.x + self.width / 2.0,
            y: self.y + self.height / 2.0,
        }
    }

    pub fn with_offset(&self, x: f64, y: f64) -> Rect {
        Rect {
            x,
            y,
            width: self.width,
            height: self.height,
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

    pub fn inflated(&self, x: f64, y: f64) -> Rect {
        Rect {
            x: self.x - x,
            y: self.y - y,
            width: self.width + 2.0 * x,
            height: self.height + 2.0 * y,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, TryFromValue, IntoValue)]
#[irondash(rename_all = "camelCase")]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Default, PartialEq, TryFromValue, IntoValue)]
#[irondash(rename_all = "camelCase")]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Default, PartialEq, TryFromValue, IntoValue)]
#[irondash(rename_all = "camelCase")]
pub struct ImageData {
    pub width: i32,
    pub height: i32,
    pub bytes_per_row: i32,
    /// Pixel data as RGBA bytes.
    pub data: Vec<u8>,
    pub device_pixel_ratio: Option<f64>,
}

impl ImageData {
    pub fn point_width(&self) -> f64 {
        self.width as f64 / self.device_pixel_ratio.unwrap_or(1.0)
    }

    pub fn point_height(&self) -> f64 {
        self.height as f64 / self.device_pixel_ratio.unwrap_or(1.0)
    }
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

#[derive(Debug, TryFromValue, IntoValue, Clone, PartialEq, Eq)]
#[irondash(tag = "type", rename_all = "camelCase")]
pub enum DataRepresentation {
    #[irondash(rename_all = "camelCase")]
    Simple { format: String, data: Value },
    #[irondash(rename_all = "camelCase")]
    Lazy {
        id: DataProviderValueId,
        format: String,
    },
    #[irondash(rename_all = "camelCase")]
    VirtualFile {
        id: DataProviderValueId,
        format: String,
        storage_suggestion: Option<VirtualFileStorage>,
    },
}

impl DataRepresentation {
    pub fn is_virtual_file(&self) -> bool {
        matches!(
            self,
            Self::VirtualFile {
                id: _,
                format: _,
                storage_suggestion: _,
            }
        )
    }
    pub fn format(&self) -> &str {
        match self {
            DataRepresentation::Simple { format, data: _ } => format,
            DataRepresentation::Lazy { id: _, format } => format,
            DataRepresentation::VirtualFile {
                id: _,
                format,
                storage_suggestion: _,
            } => format,
        }
    }
}

#[derive(Debug, TryFromValue, IntoValue, Clone, PartialEq, Eq)]
#[irondash(rename_all = "camelCase")]
pub struct DataProvider {
    pub representations: Vec<DataRepresentation>,
    pub suggested_name: Option<String>,
}

//

#[derive(Debug, TryFromValue, IntoValue, Copy, Clone, PartialEq, Eq)]
#[irondash(rename_all = "camelCase")]
pub enum VirtualFileStorage {
    TemporaryFile,
    Memory,
}

//

#[derive(TryFromValue, Debug)]
#[irondash(rename_all = "camelCase")]
pub struct TargettedImage {
    pub image_data: ImageData,
    pub rect: Rect,
}

#[derive(TryFromValue, Debug)]
#[irondash(rename_all = "camelCase")]
pub struct DragItem {
    pub data_provider_id: DataProviderId,
    /// optionally used on iPad during lifting (before dragging start)
    pub lift_image: Option<TargettedImage>,
    pub image: TargettedImage,
    pub local_data: Value,
}

#[derive(TryFromValue, Debug)]
#[irondash(rename_all = "camelCase")]
pub struct DragConfiguration {
    pub items: Vec<DragItem>,
    pub allowed_operations: Vec<DropOperation>,
    pub animates_to_starting_position_on_cancel_or_fail: bool,
    pub prefers_full_size_previews: bool,
}

impl DragConfiguration {
    pub fn get_local_data(&self) -> Vec<Value> {
        self.items.iter().map(|i| i.local_data.clone()).collect()
    }
}

#[derive(TryFromValue)]
#[irondash(rename_all = "camelCase")]
pub struct DragRequest {
    pub configuration: DragConfiguration,
    pub combined_drag_image: Option<TargettedImage>,
    pub position: Point,
}

#[derive(Debug, TryFromValue, IntoValue, Copy, Clone, PartialEq, Eq)]
#[irondash(rename_all = "camelCase")]
pub enum DropOperation {
    None,
    UserCancelled, // macOS, windows, linux - drag cancelled by user pressing escape
    Forbidden,     // Used on iOS, maps to None on other platforms
    Copy,          // macOS, iOS, Windows, Linux, Android
    Move,          // macOS, iOS (within same app), Windows, Linux
    Link,          // macOS, Windows, Linux
}

#[derive(TryFromValue, Debug)]
#[irondash(rename_all = "camelCase")]
pub struct MenuConfiguration {
    pub configuration_id: i64,
    pub preview_image: Option<ImageData>,
    pub preview_size: Option<Size>,
    pub lift_image: TargettedImage,
    pub menu_handle: i64,
    #[irondash(skip)]
    pub menu: Option<Rc<PlatformMenu>>,
}

#[derive(TryFromValue)]
#[irondash(rename_all = "camelCase")]
pub struct ShowContextMenuRequest {
    pub menu_handle: i64,
    pub location: Point,
    #[irondash(skip)]
    pub menu: Option<Rc<PlatformMenu>>,
}

#[derive(IntoValue)]
#[irondash(rename_all = "camelCase")]
pub struct ShowContextMenuResponse {
    pub item_selected: bool,
}

#[derive(TryFromValue, Debug)]
#[irondash(rename_all = "camelCase")]
pub struct DeferredMenuResponse {
    pub elements: Vec<MenuElement>,
}

#[derive(TryFromValue, Debug)]
#[irondash(rename_all = "camelCase")]
pub struct MenuActionAttributes {
    pub disabled: bool,
    pub destructive: bool,
}

#[derive(TryFromValue, Debug)]
#[irondash(rename_all = "camelCase")]
pub struct Activator {
    pub trigger: String,
    pub alt: bool,
    pub meta: bool,
    pub shift: bool,
    pub control: bool,
}

#[derive(TryFromValue, Debug, PartialEq, Eq)]
#[irondash(rename_all = "camelCase")]
pub enum MenuActionState {
    None,
    CheckOn,
    CheckOff,
    CheckMixed,
    RadioOn,
    RadioOff,
}

#[derive(TryFromValue, Debug)]
#[irondash(rename_all = "camelCase", tag = "type")]
pub enum MenuImage {
    Image { data: ImageData },
    System { name: String },
}

#[derive(TryFromValue, Debug)]
#[irondash(rename_all = "camelCase")]
pub struct Menu {
    pub unique_id: i64,
    pub identifier: Option<String>,
    pub title: Option<String>,
    pub subitle: Option<String>,
    pub image: Option<MenuImage>,
    pub children: Vec<MenuElement>,
}

#[derive(TryFromValue, Debug)]
#[irondash(rename_all = "camelCase")]
pub struct MenuAction {
    pub unique_id: i64,
    pub identifier: Option<String>,
    pub title: Option<String>,
    pub image: Option<MenuImage>,
    pub subitle: Option<String>,
    pub attributes: MenuActionAttributes,
    pub state: MenuActionState,
    pub activator: Option<Activator>,
}

#[derive(TryFromValue, Debug)]
#[irondash(rename_all = "camelCase")]
pub struct DeferredMenuElement {
    pub unique_id: i64,
}

#[derive(TryFromValue, Debug)]
#[irondash(rename_all = "camelCase")]
pub struct MenuSeparator {
    pub title: Option<String>,
}

#[derive(TryFromValue, Debug)]
#[irondash(rename_all = "camelCase", tag = "type", content = "content")]
pub enum MenuElement {
    Action(MenuAction),
    Menu(Menu),
    Deferred(DeferredMenuElement),
    Separator(MenuSeparator),
}
