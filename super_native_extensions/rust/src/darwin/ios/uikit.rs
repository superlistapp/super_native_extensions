use icrate::Foundation::{CGFloat, NSArray, NSItemProvider};
use objc2::{
    extern_class, extern_methods,
    ffi::NSInteger,
    mutability,
    rc::{Allocated, Id},
    runtime::NSObject,
    ClassType, RefEncode,
};

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIImage;

    unsafe impl ClassType for UIImage {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

pub enum _CGImage {}

unsafe impl RefEncode for _CGImage {
    const ENCODING_REF: objc2::Encoding =
        objc2::Encoding::Pointer(&objc2::Encoding::Struct("CGImage", &[]));
}

pub type UIImageOrientation = NSInteger;
pub const UIImageOrientationUp: UIImageOrientation = 0;
pub const UIImageOrientationDown: UIImageOrientation = 1;
pub const UIImageOrientationLeft: UIImageOrientation = 2;
pub const UIImageOrientationRight: UIImageOrientation = 3;
pub const UIImageOrientationUpMirrored: UIImageOrientation = 4;
pub const UIImageOrientationDownMirrored: UIImageOrientation = 5;
pub const UIImageOrientationLeftMirrored: UIImageOrientation = 6;
pub const UIImageOrientationRightMirrored: UIImageOrientation = 7;

extern_methods!(
    unsafe impl UIImage {
        #[method_id(@__retain_semantics Other imageWithCGImage:scale:orientation:)]
        pub unsafe fn imageWithCGImage_scale_orientation(
            cg_image: *const _CGImage,
            scale: CGFloat,
            orientation: UIImageOrientation,
        ) -> Id<UIImage>;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIResponder;

    unsafe impl ClassType for UIResponder {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIView;

    unsafe impl ClassType for UIView {
        #[inherits(NSObject)]
        type Super = UIResponder;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIImageView;

    unsafe impl ClassType for UIImageView {
        #[inherits(NSObject)]
        type Super = UIResponder;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_methods!(
    unsafe impl UIImageView {
        #[method_id(@__retain_semantics Init initWithImage:)]
        pub unsafe fn initWithImage(
            this: Option<Allocated<Self>>,
            image: &UIImage,
        ) -> Id<UIImageView>;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIApplication;

    unsafe impl ClassType for UIApplication {
        #[inherits(NSObject)]
        type Super = UIResponder;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_methods!(
    unsafe impl UIApplication {
        #[method_id(@__retain_semantics Other sharedApplication)]
        pub fn sharedApplication() -> Id<Self>;

        #[method(beginIgnoringInteractionEvents)]
        pub fn beginIgnoringInteractionEvents(&self);

        #[method(endIgnoringInteractionEvents)]
        pub fn endIgnoringInteractionEvents(&self);
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIPasteboard;

    unsafe impl ClassType for UIPasteboard {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_methods!(
    unsafe impl UIPasteboard {
        #[method_id(@__retain_semantics Other generalPasteboard)]
        pub fn generalPasteboard() -> Id<Self>;

        #[method(setItemProviders:)]
        pub fn setItemProviders(&self, item_providers: &NSArray<NSItemProvider>);

        #[method_id(@__retain_semantics Other itemProviders)]
        pub fn itemProviders(&self) -> Id<NSArray<NSItemProvider>>;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIDragItem;

    unsafe impl ClassType for UIDragItem {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_methods!(
    unsafe impl UIDragItem {
        #[method_id(@__retain_semantics Other itemProvider)]
        pub fn itemProvider(&self) -> Id<NSItemProvider>;
    }
);
