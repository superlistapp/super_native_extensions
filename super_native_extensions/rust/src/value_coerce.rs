use irondash_message_channel::Value;
use log::warn;

#[allow(dead_code)]
pub enum StringFormat {
    Utf8,
    Utf8NullTerminated,
    Utf16NullTerminated,
}

pub trait CoerceToData {
    fn coerce_to_data(&self, string_format: StringFormat) -> Option<Vec<u8>>;
}

impl CoerceToData for Value {
    fn coerce_to_data(&self, string_format: StringFormat) -> Option<Vec<u8>> {
        match self {
            Value::String(str) => match string_format {
                StringFormat::Utf8 => Some(str.as_bytes().to_owned()),
                StringFormat::Utf8NullTerminated => {
                    let mut data = str.as_bytes().to_owned();
                    data.push(0);
                    Some(data)
                }
                StringFormat::Utf16NullTerminated => {
                    let mut data: Vec<_> = str.encode_utf16().collect();
                    data.push(0);
                    Some(unsafe { transform_slice(&data) }.to_owned())
                }
            },
            Value::I8List(data) => Some(unsafe { transform_slice(data) }.to_owned()),
            Value::U8List(data) => Some(data.to_owned()),
            Value::I16List(data) => Some(unsafe { transform_slice(data) }.to_owned()),
            Value::U16List(data) => Some(unsafe { transform_slice(data) }.to_owned()),
            Value::I32List(data) => Some(unsafe { transform_slice(data) }.to_owned()),
            Value::U32List(data) => Some(unsafe { transform_slice(data) }.to_owned()),
            Value::I64List(data) => Some(unsafe { transform_slice(data) }.to_owned()),
            Value::F32List(data) => Some(unsafe { transform_slice(data) }.to_owned()),
            Value::F64List(data) => Some(unsafe { transform_slice(data) }.to_owned()),
            Value::Null => None,
            v => {
                warn!("Couldn't coerce {v:?} to data");
                None
            }
        }
    }
}

unsafe fn transform_slice<T>(s: &[T]) -> &[u8] {
    std::slice::from_raw_parts(s.as_ptr() as *const u8, std::mem::size_of_val(s))
}
