use nativeshell_core::{IntoValue, TryFromValue};

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
