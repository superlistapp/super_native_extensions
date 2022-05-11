use nativeshell_core::{IntoValue, TryFromValue, Value};

#[derive(Debug, TryFromValue, IntoValue, Clone, PartialEq)]
#[nativeshell(tag = "type", rename_all = "camelCase")]
pub enum ClipboardWriterItemData {
    #[nativeshell(rename_all = "camelCase")]
    Simple { types: Vec<String>, data: Value },
    #[nativeshell(rename_all = "camelCase")]
    Lazy { types: Vec<String>, id: i64 },
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
