use std::collections::HashMap;

use block::{Block, ConcreteBlock, RcBlock};
use cocoa::{
    base::{id, nil, BOOL},
    foundation::{NSArray, NSDictionary, NSInteger, NSUInteger},
};
use core_foundation::array::CFIndex;
use core_graphics::{
    base::CGFloat,
    geometry::{CGPoint, CGRect, CGSize},
};
use irondash_message_channel::{value_darwin::ValueObjcConversion, Value};
use objc::{class, msg_send, rc::StrongPtr, sel, sel_impl};

use crate::{
    api_model::{ImageData, Point, Rect, Size},
    drag_manager::DragSessionId,
    platform_impl::platform::common::{cg_image_from_image_data, to_nsdata, to_nsstring},
    util::Movable,
    value_coerce::{CoerceToData, StringFormat},
    value_promise::ValuePromiseResult,
};

impl From<CGPoint> for Point {
    fn from(p: CGPoint) -> Self {
        Point { x: p.x, y: p.y }
    }
}

impl From<Point> for CGPoint {
    fn from(p: Point) -> Self {
        CGPoint { x: p.x, y: p.y }
    }
}

impl From<CGRect> for Rect {
    fn from(r: CGRect) -> Self {
        Self {
            x: r.origin.x,
            y: r.origin.y,
            width: r.size.width,
            height: r.size.height,
        }
    }
}

impl From<Rect> for CGRect {
    fn from(r: Rect) -> Self {
        CGRect {
            origin: CGPoint { x: r.x, y: r.y },
            size: CGSize {
                width: r.width,
                height: r.height,
            },
        }
    }
}

impl From<CGSize> for Size {
    fn from(s: CGSize) -> Self {
        Size {
            width: s.width,
            height: s.height,
        }
    }
}

pub fn value_to_nsdata(value: &Value) -> StrongPtr {
    fn is_map_or_list(value: &Value) -> bool {
        matches!(value, Value::Map(_) | Value::List(_))
    }
    if is_map_or_list(value) {
        let objc = value.to_objc();
        if let Ok(objc) = objc {
            unsafe {
                #[allow(non_upper_case_globals)]
                const kCFPropertyListBinaryFormat_v1_0: CFIndex = 200;
                let data: id = msg_send![class!(NSPropertyListSerialization),
                            dataWithPropertyList:*objc
                            format:kCFPropertyListBinaryFormat_v1_0
                            options:0 as NSUInteger
                            error:nil];
                if !data.is_null() {
                    return StrongPtr::retain(data);
                }
            }
        }
    }

    let buf = value.coerce_to_data(StringFormat::Utf8);
    match buf {
        Some(data) => to_nsdata(&data),
        None => unsafe { StrongPtr::new(std::ptr::null_mut()) },
    }
}

pub fn value_promise_res_to_nsdata(value: &ValuePromiseResult) -> StrongPtr {
    match value {
        ValuePromiseResult::Ok { value } => value_to_nsdata(value),
        ValuePromiseResult::Cancelled => unsafe { StrongPtr::new(std::ptr::null_mut()) },
    }
}

// NSItemProvider utility methods

pub fn register_data_representation<F>(item_provider: id, type_identifier: &str, handler: F)
where
    F: Fn(Box<dyn Fn(id /* NSData */, id /* NSError */) + 'static + Send>) -> id + 'static + Send,
{
    let handler = Box::new(handler);
    let block = ConcreteBlock::new(move |completion_block: id| -> id {
        let completion_block = unsafe { &mut *(completion_block as *mut Block<(id, id), ()>) };
        let completion_block = unsafe { RcBlock::copy(completion_block) };
        let completion_block = unsafe { Movable::new(completion_block) };
        let completion_fn = move |data: id, err: id| {
            let completion_block = completion_block.clone();
            unsafe { completion_block.call((data, err)) };
        };
        handler(Box::new(completion_fn))
    });
    let block = block.copy();
    let type_identifier = to_nsstring(type_identifier);
    unsafe {
        let () = msg_send![item_provider,
            registerDataRepresentationForTypeIdentifier:*type_identifier
            visibility: 0 as NSUInteger // all
            loadHandler: &*block];
    }
}

pub fn register_file_representation<F>(
    item_provider: id,
    type_identifier: &str,
    open_in_place: bool,
    handler: F,
) where
    F: Fn(
            Box<dyn Fn(id /* NSURL */, bool /* coordinated */, id /* NSError */) + 'static + Send>,
        ) -> id /* NSProgress */
        + 'static
        + Send,
{
    let handler = Box::new(handler);
    let block = ConcreteBlock::new(move |completion_block: id| -> id {
        let completion_block =
            unsafe { &mut *(completion_block as *mut Block<(id, BOOL, id), ()>) };
        let completion_block = unsafe { RcBlock::copy(completion_block) };
        let completion_block = unsafe { Movable::new(completion_block) };
        let completion_fn = move |data: id, coordinated: bool, err: id| {
            let completion_block = completion_block.clone();
            unsafe { completion_block.call((data, coordinated as BOOL, err)) };
        };
        handler(Box::new(completion_fn))
    });
    let block = block.copy();
    let type_identifier = to_nsstring(type_identifier);
    unsafe {
        let () = msg_send![item_provider,
            registerFileRepresentationForTypeIdentifier:*type_identifier
            fileOptions: i32::from(open_in_place) as NSInteger
            visibility: 0 as NSUInteger // all
            loadHandler: &*block
        ];
    }
}

pub trait IntoObjc {
    fn into_objc(self) -> StrongPtr;
}

impl IntoObjc for HashMap<StrongPtr, StrongPtr> {
    fn into_objc(self) -> StrongPtr {
        let keys: Vec<_> = self.keys().map(|k| k.clone().autorelease()).collect();
        let objects: Vec<_> = self.values().map(|o| o.clone().autorelease()).collect();
        unsafe {
            StrongPtr::retain(NSDictionary::dictionaryWithObjects_forKeys_(
                nil,
                NSArray::arrayWithObjects(nil, &objects),
                NSArray::arrayWithObjects(nil, &keys),
            ))
        }
    }
}

impl IntoObjc for HashMap<&str, StrongPtr> {
    fn into_objc(self) -> StrongPtr {
        let keys: Vec<_> = self.keys().map(|k| to_nsstring(k).autorelease()).collect();
        let objects: Vec<_> = self.values().map(|o| o.clone().autorelease()).collect();
        unsafe {
            StrongPtr::retain(NSDictionary::dictionaryWithObjects_forKeys_(
                nil,
                NSArray::arrayWithObjects(nil, &objects),
                NSArray::arrayWithObjects(nil, &keys),
            ))
        }
    }
}

impl IntoObjc for i64 {
    fn into_objc(self) -> StrongPtr {
        unsafe { StrongPtr::retain(msg_send![class!(NSNumber), numberWithLongLong: self]) }
    }
}

impl IntoObjc for DragSessionId {
    fn into_objc(self) -> StrongPtr {
        let id: i64 = self.into();
        id.into_objc()
    }
}

pub fn image_view_from_data(image_data: ImageData) -> StrongPtr {
    let orientation_up: NSInteger = 0; // need to flip CGImage
    let pixel_ratio = image_data.device_pixel_ratio;
    let image = cg_image_from_image_data(image_data);
    let image: id = unsafe {
        msg_send![class!(UIImage),
        imageWithCGImage: &*image
        scale: pixel_ratio.unwrap_or(1.0) as CGFloat
        orientation: orientation_up]
    };
    let image_view: id = unsafe { msg_send![class!(UIImageView), alloc] };
    unsafe { StrongPtr::new(msg_send![image_view, initWithImage: image]) }
}
