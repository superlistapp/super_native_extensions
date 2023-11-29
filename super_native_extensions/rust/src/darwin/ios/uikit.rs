use icrate::{
    block2::Block,
    Foundation::{CGFloat, CGPoint, CGRect, NSArray, NSItemProvider},
};
use objc2::{
    extern_class, extern_methods, extern_protocol,
    ffi::{NSInteger, NSUInteger},
    mutability,
    rc::{Allocated, Id},
    runtime::{Bool, NSObject, NSObjectProtocol, ProtocolObject},
    ClassType, ProtocolType, RefEncode,
};

pub type UIDropOperation = NSUInteger;

pub const UIDropOperationCancel: UIDropOperation = 0;
pub const UIDropOperationForbidden: UIDropOperation = 1;
pub const UIDropOperationCopy: UIDropOperation = 2;
pub const UIDropOperationMove: UIDropOperation = 3;

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

extern_methods!(
    unsafe impl UIView {
        #[method(bounds)]
        pub fn bounds(&self) -> CGRect;

        #[method(setBounds:)]
        pub fn setBounds(&self, value: CGRect);

        #[method(frame)]
        pub fn frame(&self) -> CGRect;

        #[method(setFrame:)]
        pub fn setFrame(&self, value: CGRect);

        #[method(setUserInteractionEnabled:)]
        pub fn setUserInteractionEnabled(&self, enabled: bool);

        #[method(userInteractionEnabled)]
        pub fn userInteractionEnabled(&self) -> bool;

        #[method_id(@__retain_semantics Init initWithFrame:)]
        pub fn initWithFrame(this: Option<Allocated<Self>>, frame: CGRect) -> Id<Self>;

        #[method(addSubview:)]
        pub fn addSubview(&self, subview: &UIView);

        #[method(setAlpha:)]
        pub fn setAlpha(&self, alpha: CGFloat);

        #[method(removeFromSuperview)]
        pub fn removeFromSuperview(&self);

        #[method(addInteraction:)]
        pub fn addInteraction(&self, interaction: &NSObject);

        #[method(removeInteraction:)]
        pub fn removeInteraction(&self, interaction: &NSObject);

        #[method(animateWithDuration:delay:options:animations:completion:)]
        pub fn animateWithDuration_delay_options_animations_completion(
            duration: f64,
            delay: f64,
            options: UIViewAnimationOptions,
            animations: &Block<(), ()>,
            completion: Option<&Block<(Bool,), ()>>,
        );
    }
);

pub type UIViewAnimationOptions = NSInteger;
pub const UIViewAnimationOptionNone: UIViewAnimationOptions = 0;

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIImageView;

    unsafe impl ClassType for UIImageView {
        #[inherits(NSObject)]
        type Super = UIView;
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
        #[method_id(@__retain_semantics Init initWithItemProvider:)]
        pub fn initWithItemProvider(
            this: Option<Allocated<Self>>,
            provider: &NSItemProvider,
        ) -> Id<Self>;

        #[method_id(@__retain_semantics Other itemProvider)]
        pub fn itemProvider(&self) -> Id<NSItemProvider>;

        #[method(setLocalObject:)]
        pub fn setLocalObject(&self, object: Option<&NSObject>);

        #[method_id(@__retain_semantics Other localObject)]
        pub fn localObject(&self) -> Option<Id<NSObject>>;

        #[method(setPreviewProvider:)]
        pub fn setPreviewProvider(&self, provider: Option<&Block<(), *mut UIDragPreview>>);

        #[method(previewProvider)]
        pub fn previewProvider(&self) -> Option<&Block<(), *mut UIDragPreview>>;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIDragPreview;

    unsafe impl ClassType for UIDragPreview {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_methods!(
    unsafe impl UIDragPreview {
        #[method_id(@__retain_semantics Init initWithView:parameters:)]
        pub fn initWithView_parameters(
            this: Option<Allocated<Self>>,
            view: &UIView,
            parameters: &UIDragPreviewParameters,
        ) -> Id<Self>;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIPreviewTarget;

    unsafe impl ClassType for UIPreviewTarget {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_methods!(
    unsafe impl UIPreviewTarget {
        #[method_id(@__retain_semantics Init initWithContainer:center:)]
        pub fn initWithContainer_center(
            this: Option<Allocated<Self>>,
            container: &UIView,
            center: CGPoint,
        ) -> Id<Self>;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UITargetedPreview;

    unsafe impl ClassType for UITargetedPreview {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UITargetedDragPreview;

    unsafe impl ClassType for UITargetedDragPreview {
        type Super = UITargetedPreview;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_methods!(
    unsafe impl UITargetedDragPreview {
        #[method_id(@__retain_semantics Init initWithView:parameters:target:)]
        pub fn initWithView_parameters_target(
            this: Option<Allocated<Self>>,
            view: &UIView,
            parameters: &UIDragPreviewParameters,
            target: &UIPreviewTarget,
        ) -> Id<Self>;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIDragPreviewParameters;

    unsafe impl ClassType for UIDragPreviewParameters {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_methods!(
    unsafe impl UIDragPreviewParameters {
        #[method_id(@__retain_semantics Init init)]
        pub fn init(this: Option<Allocated<Self>>) -> Id<Self>;

        #[method(setBackgroundColor:)]
        pub fn setBackgroundColor(&self, color: Option<&UIColor>);

        #[method(setShadowPath:)]
        pub fn setShadowPath(&self, path: Option<&UIBezierPath>);
    }
);

extern_protocol!(
    pub unsafe trait UIDragDropSession: NSObjectProtocol {
        #[method_id(@__retain_semantics Other items)]
        fn items(&self) -> Id<NSArray<UIDragItem>>;
    }

    unsafe impl ProtocolType for dyn UIDragDropSession {}
);

extern_protocol!(
    pub unsafe trait UIDragSession: UIDragDropSession {
        #[method(setLocalContext:)]
        fn setLocalContext(&self, context: Option<&NSObject>);

        #[method_id(@__retain_semantics Other localContext)]
        fn localContext(&self) -> Option<Id<NSObject>>;

        #[method(locationInView:)]
        fn locationInView(&self, view: &UIView) -> CGPoint;
    }

    unsafe impl ProtocolType for dyn UIDragSession {}
);

extern_protocol!(
    pub unsafe trait UIDragAnimating: NSObjectProtocol {}

    unsafe impl ProtocolType for dyn UIDragAnimating {}
);

extern_protocol!(
    pub unsafe trait UIDragInteractionDelegate: NSObjectProtocol {
        #[method_id(@__retain_semantics Other dragInteraction:itemsForBeginningSession:)]
        fn dragInteraction_itemsForBeginningSession(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
        ) -> Id<NSArray<UIDragItem>>;

        #[optional]
        #[method_id(@__retain_semantics Other dragInteraction:itemsForAddingToSession:withTouchAtPoint:)]
        fn dragInteraction_itemsForAddingToSession_withTouchAtPoint(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
            point: CGPoint,
        ) -> Id<NSArray<UIDragItem>>;

        #[optional]
        #[method_id(@__retain_semantics Other dragInteraction:sessionForAddingItems:withTouchAtPoint:)]
        fn dragInteraction_sessionForAddingItems_withTouchAtPoint(
            &self,
            interaction: &UIDragInteraction,
            sessions: &NSArray<ProtocolObject<dyn UIDragSession>>,
            point: CGPoint,
        ) -> Id<NSObject>;

        #[optional]
        #[method(dragInteraction:willAnimateLiftWithAnimator:session:)]
        fn dragInteraction_willAnimateLiftWithAnimator_session(
            &self,
            interaction: &UIDragInteraction,
            animator: &ProtocolObject<dyn UIDragAnimating>,
            session: &ProtocolObject<dyn UIDragSession>,
        );

        #[optional]
        #[method(dragInteraction:item:willAnimateCancelWithAnimator:)]
        fn dragInteraction_item_willAnimateCancelWithAnimator(
            &self,
            interaction: &UIDragInteraction,
            item: &UIDragItem,
            animator: &ProtocolObject<dyn UIDragAnimating>,
        );

        #[optional]
        #[method(dragInteraction:sessionWillBegin:)]
        fn dragInteraction_sessionWillBegin(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
        );

        #[optional]
        #[method(dragInteraction:session:willAddItems:forInteraction:)]
        fn dragInteraction_session_willAddItems_forInteraction(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
            items: &NSArray<UIDragItem>,
            adding_interaction: &UIDragInteraction,
        );

        #[optional]
        #[method(dragInteraction:sessionDidMove:)]
        fn dragInteraction_sessionDidMove(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
        );

        #[optional]
        #[method(dragInteraction:session:willEndWithOperation:)]
        fn dragInteraction_session_willEndWithOperation(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
            operation: UIDropOperation,
        );

        #[optional]
        #[method(dragInteraction:session:didEndWithOperation:)]
        fn dragInteraction_session_didEndWithOperation(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
            operation: UIDropOperation,
        );

        #[optional]
        #[method(dragInteraction:sessionDidTransferItems:)]
        fn dragInteraction_sessionDidTransferItems(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
        );

        #[optional]
        #[method_id(@__retain_semantics Other dragInteraction:previewForLiftingItem:session:)]
        fn dragInteraction_previewForLiftingItem_session(
            &self,
            interaction: &UIDragInteraction,
            item: &UIDragItem,
            session: &ProtocolObject<dyn UIDragSession>,
        ) -> Option<Id<UITargetedDragPreview>>;

        #[optional]
        #[method_id(@__retain_semantics Other dragInteraction:previewForCancellingItem:withDefault:)]
        fn dragInteraction_previewForCancellingItem_withDefault(
            &self,
            interaction: &UIDragInteraction,
            item: &UIDragItem,
            default_preview: &UITargetedDragPreview,
        ) -> Option<Id<UITargetedDragPreview>>;

        #[optional]
        #[method(dragInteraction:prefersFullSizePreviewsForSession:)]
        fn dragInteraction_prefersFullSizePreviewsForSession(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
        ) -> bool;

        #[optional]
        #[method(dragInteraction:sessionIsRestrictedToDraggingApplication:)]
        fn dragInteraction_sessionIsRestrictedToDraggingApplication(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
        ) -> bool;

        #[optional]
        #[method(dragInteraction:sessionAllowsMoveOperation:)]
        fn dragInteraction_sessionAllowsMoveOperation(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
        ) -> bool;
    }

    unsafe impl ProtocolType for dyn UIDragInteractionDelegate {}
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIDragInteraction;

    unsafe impl ClassType for UIDragInteraction {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_methods!(
    unsafe impl UIDragInteraction {
        #[method_id(@__retain_semantics Init initWithDelegate:)]
        pub unsafe fn initWithDelegate(
            this: Option<Allocated<Self>>,
            delegate: &ProtocolObject<dyn UIDragInteractionDelegate>,
        ) -> Id<Self>;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIColor;

    unsafe impl ClassType for UIColor {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_methods!(
    unsafe impl UIColor {
        #[method_id(@__retain_semantics Other clearColor)]
        pub fn clearColor() -> Id<Self>;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIBezierPath;

    unsafe impl ClassType for UIBezierPath {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_methods!(
    unsafe impl UIBezierPath {
        #[method_id(@__retain_semantics Other bezierPath)]
        pub fn bezierPath() -> Id<Self>;

        #[method_id(@__retain_semantics Other bezierPathWithRect:)]
        pub fn bezierPathWithRect(rect: CGRect) -> Id<Self>;

        #[method(appendPath:)]
        pub fn appendPath(&self, path: &UIBezierPath);
    }
);
