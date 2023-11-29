use std::{collections::HashMap, ops::Deref, ptr::NonNull};

use icrate::{
    block2::{Block, ConcreteBlock, RcBlock},
    Foundation::{
        CGPoint, CGRect, CGSize, NSData, NSDictionary, NSError, NSItemProvider,
        NSItemProviderFileOptionOpenInPlace, NSItemProviderRepresentationVisibilityAll, NSNumber,
        NSProgress, NSPropertyListBinaryFormat_v1_0, NSPropertyListSerialization, NSString, NSURL,
    },
};
use irondash_message_channel::{value_darwin::ValueObjcConversion, Value};
use objc2::{
    rc::Id,
    runtime::{Bool, NSObject},
    ClassType,
};

use crate::{
    api_model::{ImageData, Point, Rect, Size},
    drag_manager::DragSessionId,
    platform_impl::platform::common::cg_image_from_image_data,
    util::Movable,
    value_coerce::{CoerceToData, StringFormat},
    value_promise::ValuePromiseResult,
};

use super::uikit::{UIApplication, UIImage, UIImageOrientationUp, UIImageView, _CGImage};

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

impl From<Size> for CGSize {
    fn from(s: Size) -> Self {
        CGSize {
            width: s.width,
            height: s.height,
        }
    }
}

pub fn value_to_nsdata(value: &Value) -> Option<Id<NSData>> {
    fn is_map_or_list(value: &Value) -> bool {
        matches!(value, Value::Map(_) | Value::List(_))
    }
    if is_map_or_list(value) {
        let objc = value.to_objc();
        if let Ok(Some(objc)) = objc {
            let data = unsafe {
                NSPropertyListSerialization::dataWithPropertyList_format_options_error(
                    &objc,
                    NSPropertyListBinaryFormat_v1_0,
                    0,
                )
            };
            if let Ok(data) = data {
                return Some(data);
            }
        }
    }

    let buf = value.coerce_to_data(StringFormat::Utf8);
    buf.map(NSData::from_vec)
}

pub fn value_promise_res_to_nsdata(value: &ValuePromiseResult) -> Option<Id<NSData>> {
    match value {
        ValuePromiseResult::Ok { value } => value_to_nsdata(value),
        ValuePromiseResult::Cancelled => None,
    }
}

// NSItemProvider utility methods

pub fn register_data_representation<F>(
    item_provider: &NSItemProvider,
    type_identifier: &str,
    handler: F,
) where
    F: Fn(
            Box<dyn Fn(Option<&NSData>, Option<&NSError>) + 'static + Send>,
        ) -> Option<Id<NSProgress>>
        + 'static
        + Send,
{
    let block = ConcreteBlock::new(move |completion_block: NonNull<Block<(*mut NSData, *mut NSError), ()>>| -> *mut NSProgress {
        let completion_block = unsafe { RcBlock::copy(completion_block.as_ptr())};
        let completion_block = unsafe { Movable::new(completion_block) };
        let completion_fn = move |data: Option<&NSData>, err: Option<&NSError>| {
            let completion_block = completion_block.clone();
            let data = data.map(|d| d as * const _ as * mut _).unwrap_or(std::ptr::null_mut());
            let err = err.map(|e| e as *const _ as * mut _).unwrap_or(std::ptr::null_mut());
            unsafe { completion_block.call((data, err)) };
        };
        let res = handler(Box::new(completion_fn));
        res.map(Id::autorelease_return).unwrap_or(std::ptr::null_mut())
    });
    let block = block.copy();
    unsafe {
        item_provider.registerDataRepresentationForTypeIdentifier_visibility_loadHandler(
            &NSString::from_str(type_identifier),
            NSItemProviderRepresentationVisibilityAll,
            &block,
        )
    }
}

pub fn register_file_representation<F>(
    item_provider: &NSItemProvider,
    type_identifier: &str,
    open_in_place: bool,
    handler: F,
) where
    F: Fn(
            Box<dyn Fn(Option<&NSURL>, bool /* coordinated */, Option<&NSError>) + 'static + Send>,
        ) -> Option<Id<NSProgress>>
        + 'static
        + Send,
{
    let block = ConcreteBlock::new(move |completion_block: NonNull<Block<(*mut NSURL, Bool, *mut NSError), ()>>| -> *mut NSProgress {
        let completion_block = unsafe { RcBlock::copy(completion_block.as_ptr())};
        let completion_block = unsafe { Movable::new(completion_block) };
        let completion_fn = move |data: Option<&NSURL>, coordinated: bool, err: Option<&NSError>| {
            let completion_block = completion_block.clone();
            let data = data.map(|d| d as * const _ as * mut _).unwrap_or(std::ptr::null_mut());
            let err = err.map(|e| e as *const _ as * mut _).unwrap_or(std::ptr::null_mut());
            unsafe { completion_block.call((data, coordinated.into(), err)) };
        };
        let res = handler(Box::new(completion_fn));
        res.map(Id::autorelease_return).unwrap_or(std::ptr::null_mut())
    });
    let block = block.copy();
    unsafe {
        item_provider
            .registerFileRepresentationForTypeIdentifier_fileOptions_visibility_loadHandler(
                &NSString::from_str(type_identifier),
                if open_in_place {
                    NSItemProviderFileOptionOpenInPlace
                } else {
                    0
                },
                NSItemProviderRepresentationVisibilityAll,
                &block,
            )
    }
}

pub trait IntoObjc {
    fn into_objc(self) -> Id<NSObject>;
}

// impl IntoObjc for HashMap<StrongPtr, StrongPtr> {
//     fn into_objc(self) -> StrongPtr {
//         let keys: Vec<_> = self.keys().map(|k| k.clone().autorelease()).collect();
//         let objects: Vec<_> = self.values().map(|o| o.clone().autorelease()).collect();
//         unsafe {
//             StrongPtr::retain(NSDictionary::dictionaryWithObjects_forKeys_(
//                 nil,
//                 NSArray::arrayWithObjects(nil, &objects),
//                 NSArray::arrayWithObjects(nil, &keys),
//             ))
//         }
//     }
// }

impl IntoObjc for HashMap<&str, Id<NSObject>> {
    fn into_objc(mut self) -> Id<NSObject> {
        let mut keys = Vec::<Id<NSString>>::new();
        let mut objects = Vec::<Id<NSObject>>::new();
        for (k, v) in self.drain() {
            keys.push(NSString::from_str(k));
            objects.push(v);
        }
        let keys = keys.iter().map(|k| k.deref()).collect::<Vec<_>>();
        unsafe {
            let res = NSDictionary::from_keys_and_objects(&keys, objects);
            Id::cast(res)
        }
    }
}

impl IntoObjc for i64 {
    fn into_objc(self) -> Id<NSObject> {
        unsafe {
            let res = NSNumber::numberWithLongLong(self);
            Id::cast(res)
        }
    }
}

impl IntoObjc for DragSessionId {
    fn into_objc(self) -> Id<NSObject> {
        let id: i64 = self.into();
        id.into_objc()
    }
}

pub fn image_from_image_data(image_data: ImageData) -> Id<UIImage> {
    let pixel_ratio = image_data.device_pixel_ratio;
    let image = cg_image_from_image_data(image_data);
    let image = &*image as *const _ as *const _CGImage;
    unsafe {
        UIImage::imageWithCGImage_scale_orientation(
            image,
            pixel_ratio.unwrap_or(1.0),
            UIImageOrientationUp,
        )
    }
}

pub fn image_view_from_data(image_data: ImageData) -> Id<UIImageView> {
    let image = image_from_image_data(image_data);
    unsafe { UIImageView::initWithImage(UIImageView::alloc(), &image) }
}

/// Ignores the notifications event while in scope.
pub struct IgnoreInteractionEvents {}

impl IgnoreInteractionEvents {
    pub fn new() -> Self {
        unsafe {
            // beginIgnoringInteractionEvents is a big stick but we need one
            // to prevent active drag gesture recognizer from getting events while
            // waiting for drag data.
            let application = UIApplication::sharedApplication();
            application.beginIgnoringInteractionEvents();
        }
        Self {}
    }
}

impl Drop for IgnoreInteractionEvents {
    fn drop(&mut self) {
        unsafe {
            let application = UIApplication::sharedApplication();
            application.endIgnoringInteractionEvents();
        }
    }
}
