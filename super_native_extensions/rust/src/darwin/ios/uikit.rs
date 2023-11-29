use std::ptr::NonNull;

use icrate::{
    block2::Block,
    Foundation::{
        CGFloat, CGPoint, CGRect, CGSize, NSArray, NSItemProvider, NSString, NSTimeInterval,
    },
};
use objc2::{
    extern_class, extern_methods, extern_protocol,
    ffi::{NSInteger, NSUInteger},
    mutability,
    rc::{Allocated, Id},
    runtime::{Bool, NSObject, NSObjectProtocol, ProtocolObject},
    ClassType, ProtocolType, RefEncode,
};

use crate::platform_impl::platform::common::CGAffineTransform;

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

extern_methods!(
    unsafe impl UIImage {
        #[method_id(@__retain_semantics Other imageWithCGImage:scale:orientation:)]
        pub unsafe fn imageWithCGImage_scale_orientation(
            cg_image: *const _CGImage,
            scale: CGFloat,
            orientation: UIImageOrientation,
        ) -> Id<UIImage>;

        #[method_id(@__retain_semantics Other systemImageNamed:)]
        pub unsafe fn systemImageNamed(name: &NSString) -> Option<Id<UIImage>>;
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
        pub unsafe fn bounds(&self) -> CGRect;

        #[method(setBounds:)]
        pub unsafe fn setBounds(&self, value: CGRect);

        #[method(frame)]
        pub unsafe fn frame(&self) -> CGRect;

        #[method(setFrame:)]
        pub unsafe fn setFrame(&self, value: CGRect);

        #[method(setUserInteractionEnabled:)]
        pub unsafe fn setUserInteractionEnabled(&self, enabled: bool);

        #[method(userInteractionEnabled)]
        pub unsafe fn userInteractionEnabled(&self) -> bool;

        #[method_id(@__retain_semantics Init initWithFrame:)]
        pub unsafe fn initWithFrame(this: Option<Allocated<Self>>, frame: CGRect) -> Id<Self>;

        #[method(addSubview:)]
        pub unsafe fn addSubview(&self, subview: &UIView);

        #[method_id(@__retain_semantics Other subviews)]
        pub unsafe fn subviews(&self) -> Id<NSArray<UIView>>;

        #[method(setAlpha:)]
        pub unsafe fn setAlpha(&self, alpha: CGFloat);

        #[method(alpha)]
        pub unsafe fn alpha(&self) -> CGFloat;

        #[method(setCenter:)]
        pub unsafe fn setCenter(&self, center: CGPoint);

        #[method(center)]
        pub unsafe fn center(&self) -> CGPoint;

        #[method(removeFromSuperview)]
        pub unsafe fn removeFromSuperview(&self);

        #[method(addInteraction:)]
        pub unsafe fn addInteraction(&self, interaction: &NSObject);

        #[method(removeInteraction:)]
        pub unsafe fn removeInteraction(&self, interaction: &NSObject);

        #[method(animateWithDuration:delay:options:animations:completion:)]
        pub unsafe fn animateWithDuration_delay_options_animations_completion(
            duration: NSTimeInterval,
            delay: NSTimeInterval,
            options: UIViewAnimationOptions,
            animations: &Block<(), ()>,
            completion: Option<&Block<(Bool,), ()>>,
        );

        #[method(animateWithDuration:animations:completion:)]
        pub unsafe fn animateWithDuration_animations_completion(
            duration: f64,
            animations: &Block<(), ()>,
            completion: Option<&Block<(Bool,), ()>>,
        );
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIActivityIndicatorView;

    unsafe impl ClassType for UIActivityIndicatorView {
        #[inherits(NSObject)]
        type Super = UIView;
        type Mutability = mutability::InteriorMutable;
    }
);

pub type UIActivityIndicatorViewStyle = NSInteger;
pub const UIActivityIndicatorViewStyleWhiteLarge: UIActivityIndicatorViewStyle = 0;
pub const UIActivityIndicatorViewStyleWhite: UIActivityIndicatorViewStyle = 1;
pub const UIActivityIndicatorViewStyleGray: UIActivityIndicatorViewStyle = 2;
pub const UIActivityIndicatorViewStyleMedium: UIActivityIndicatorViewStyle = 100;
pub const UIActivityIndicatorViewStyleLarge: UIActivityIndicatorViewStyle = 101;

extern_methods!(
    unsafe impl UIActivityIndicatorView {
        #[method_id(@__retain_semantics Init initWithActivityIndicatorStyle:)]
        pub unsafe fn initWithActivityIndicatorStyle(
            this: Option<Allocated<Self>>,
            style: UIActivityIndicatorViewStyle,
        ) -> Id<Self>;

        #[method(startAnimating)]
        pub unsafe fn startAnimating(&self);

        #[method(stopAnimating)]
        pub unsafe fn stopAnimating(&self);

        #[method(setColor:)]
        pub unsafe fn setColor(&self, color: Option<&UIColor>);

        #[method_id(@__retain_semantics Other color)]
        pub unsafe fn color(&self) -> Option<Id<UIColor>>;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIViewController;

    unsafe impl ClassType for UIViewController {
        #[inherits(NSObject)]
        type Super = UIResponder;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_methods!(
    unsafe impl UIViewController {
        #[method_id(@__retain_semantics Other view)]
        pub unsafe fn view(&self) -> Option<Id<UIView>>;

        #[method(setView:)]
        pub unsafe fn setView(&self, view: Option<&UIView>);

        #[method(setPreferredContentSize:)]
        pub unsafe fn setPreferredContentSize(&self, size: CGSize);

        #[method(preferredContentSize)]
        pub unsafe fn preferredContentSize(&self) -> CGSize;

        #[method_id(@__retain_semantics Init init)]
        pub unsafe fn init(this: Option<Allocated<Self>>) -> Id<Self>;
    }
);

pub type UIViewAnimationOptions = NSUInteger;
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
        pub unsafe fn sharedApplication() -> Id<Self>;

        #[method(beginIgnoringInteractionEvents)]
        pub unsafe fn beginIgnoringInteractionEvents(&self);

        #[method(endIgnoringInteractionEvents)]
        pub unsafe fn endIgnoringInteractionEvents(&self);
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
        pub unsafe fn generalPasteboard() -> Id<Self>;

        #[method(setItemProviders:)]
        pub unsafe fn setItemProviders(&self, item_providers: &NSArray<NSItemProvider>);

        #[method_id(@__retain_semantics Other itemProviders)]
        pub unsafe fn itemProviders(&self) -> Id<NSArray<NSItemProvider>>;
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
        pub unsafe fn initWithItemProvider(
            this: Option<Allocated<Self>>,
            provider: &NSItemProvider,
        ) -> Id<Self>;

        #[method_id(@__retain_semantics Other itemProvider)]
        pub unsafe fn itemProvider(&self) -> Id<NSItemProvider>;

        #[method(setLocalObject:)]
        pub unsafe fn setLocalObject(&self, object: Option<&NSObject>);

        #[method_id(@__retain_semantics Other localObject)]
        pub unsafe fn localObject(&self) -> Option<Id<NSObject>>;

        #[method(setPreviewProvider:)]
        pub unsafe fn setPreviewProvider(&self, provider: Option<&Block<(), *mut UIDragPreview>>);

        #[method(previewProvider)]
        pub unsafe fn previewProvider(&self) -> Option<&Block<(), *mut UIDragPreview>>;
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
        pub unsafe fn initWithView_parameters(
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
        pub unsafe fn initWithContainer_center(
            this: Option<Allocated<Self>>,
            container: &UIView,
            center: CGPoint,
        ) -> Id<Self>;

        #[method_id(@__retain_semantics Init initWithContainer:center:transform:)]
        pub unsafe fn initWithContainer_center_transform(
            this: Option<Allocated<Self>>,
            container: &UIView,
            center: CGPoint,
            transform: CGAffineTransform,
        ) -> Id<Self>;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIDragPreviewTarget;

    unsafe impl ClassType for UIDragPreviewTarget {
        type Super = UIPreviewTarget;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_methods!(
    unsafe impl UIDragPreviewTarget {
        #[method_id(@__retain_semantics Init initWithContainer:center:)]
        pub unsafe fn initWithContainer_center(
            this: Option<Allocated<Self>>,
            container: &UIView,
            center: CGPoint,
        ) -> Id<Self>;

        #[method_id(@__retain_semantics Init initWithContainer:center:transform:)]
        pub unsafe fn initWithContainer_center_transform(
            this: Option<Allocated<Self>>,
            container: &UIView,
            center: CGPoint,
            transform: CGAffineTransform,
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

extern_methods!(
    unsafe impl UITargetedPreview {
        #[method_id(@__retain_semantics Init initWithView:parameters:target:)]
        pub unsafe fn initWithView_parameters_target(
            this: Option<Allocated<Self>>,
            view: &UIView,
            parameters: &UIPreviewParameters,
            target: &UIPreviewTarget,
        ) -> Id<Self>;

        #[method_id(@__retain_semantics Other view)]
        pub unsafe fn view(&self) -> Id<UIView>;

        #[method(size)]
        pub unsafe fn size(&self) -> CGSize;

        #[method_id(@__retain_semantics Other parameters)]
        pub unsafe fn parameters(&self) -> Id<UIPreviewParameters>;
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
        pub unsafe fn initWithView_parameters_target(
            this: Option<Allocated<Self>>,
            view: &UIView,
            parameters: &UIDragPreviewParameters,
            target: &UIPreviewTarget,
        ) -> Id<Self>;

        #[method_id(@__retain_semantics Other retargetedPreviewWithTarget:)]
        pub unsafe fn retargetedPreviewWithTarget(
            &self,
            target: &UIPreviewTarget,
        ) -> Id<UITargetedDragPreview>;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIPreviewParameters;

    unsafe impl ClassType for UIPreviewParameters {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIDragPreviewParameters;

    unsafe impl ClassType for UIDragPreviewParameters {
        type Super = UIPreviewParameters;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_methods!(
    unsafe impl UIPreviewParameters {
        #[method_id(@__retain_semantics Init init)]
        pub unsafe fn init(this: Option<Allocated<Self>>) -> Id<Self>;

        #[method(setBackgroundColor:)]
        pub unsafe fn setBackgroundColor(&self, color: Option<&UIColor>);

        #[method(setShadowPath:)]
        pub unsafe fn setShadowPath(&self, path: Option<&UIBezierPath>);
    }
);

extern_methods!(
    unsafe impl UIDragPreviewParameters {
        #[method_id(@__retain_semantics Init init)]
        pub unsafe fn init(this: Option<Allocated<Self>>) -> Id<Self>;
    }
);

extern_protocol!(
    pub unsafe trait UIDragDropSession: NSObjectProtocol {
        #[method_id(@__retain_semantics Other items)]
        unsafe fn items(&self) -> Id<NSArray<UIDragItem>>;

        #[method(locationInView:)]
        unsafe fn locationInView(&self, view: &UIView) -> CGPoint;

        #[method(allowsMoveOperation)]
        unsafe fn allowsMoveOperation(&self) -> bool;
    }

    unsafe impl ProtocolType for dyn UIDragDropSession {}
);

extern_protocol!(
    pub unsafe trait UIDragSession: UIDragDropSession {
        #[method(setLocalContext:)]
        unsafe fn setLocalContext(&self, context: Option<&NSObject>);

        #[method_id(@__retain_semantics Other localContext)]
        unsafe fn localContext(&self) -> Option<Id<NSObject>>;
    }

    unsafe impl ProtocolType for dyn UIDragSession {}
);

pub type UIDropSessionProgressIndicatorStyle = NSUInteger;
pub const UIDropSessionProgressIndicatorStyleNone: UIDropSessionProgressIndicatorStyle = 0;
pub const UIDropSessionProgressIndicatorStyleDefault: UIDropSessionProgressIndicatorStyle = 1;

extern_protocol!(
    pub unsafe trait UIDropSession: UIDragDropSession {
        #[method_id(@__retain_semantics Other localDragSession)]
        unsafe fn localDragSession(&self) -> Option<Id<ProtocolObject<dyn UIDragSession>>>;

        #[method(progressIndicatorStyle)]
        unsafe fn progressIndicatorStyle(&self) -> UIDropSessionProgressIndicatorStyle;

        #[method(setProgressIndicatorStyle:)]
        unsafe fn setProgressIndicatorStyle(&self, style: UIDropSessionProgressIndicatorStyle);
    }

    unsafe impl ProtocolType for dyn UIDropSession {}
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIDropProposal;

    unsafe impl ClassType for UIDropProposal {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_methods!(
    unsafe impl UIDropProposal {
        #[method_id(@__retain_semantics Init initWithDropOperation:)]
        pub unsafe fn initWithDropOperation(
            this: Option<Allocated<Self>>,
            operation: UIDropOperation,
        ) -> Id<Self>;

        #[method(operation)]
        pub unsafe fn operation(&self) -> UIDropOperation;

        #[method(setPrecise:)]
        pub unsafe fn setPrecise(&self, precise: bool);

        #[method(isPrecise)]
        pub unsafe fn precise(&self) -> bool;

        #[method(setPrefersFullSizePreview:)]
        pub unsafe fn setPrefersFullSizePreview(&self, prefers: bool);

        #[method(prefersFullSizePreview)]
        pub unsafe fn prefersFullSizePreview(&self) -> bool;
    }
);

extern_protocol!(
    pub unsafe trait UIDragAnimating: NSObjectProtocol {}

    unsafe impl ProtocolType for dyn UIDragAnimating {}
);

extern_protocol!(
    pub unsafe trait UIContextMenuInteractionAnimating: NSObjectProtocol {
        #[method(addAnimations:)]
        unsafe fn addAnimations(&self, animations: &Block<(), ()>);

        #[method(addCompletion:)]
        unsafe fn addCompletion(&self, completion: &Block<(), ()>);
    }

    unsafe impl ProtocolType for dyn UIContextMenuInteractionAnimating {}
);

extern_protocol!(
    pub unsafe trait UIDragInteractionDelegate: NSObjectProtocol {
        #[method_id(@__retain_semantics Other dragInteraction:itemsForBeginningSession:)]
        unsafe fn dragInteraction_itemsForBeginningSession(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
        ) -> Id<NSArray<UIDragItem>>;

        #[optional]
        #[method_id(@__retain_semantics Other dragInteraction:itemsForAddingToSession:withTouchAtPoint:)]
        unsafe fn dragInteraction_itemsForAddingToSession_withTouchAtPoint(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
            point: CGPoint,
        ) -> Id<NSArray<UIDragItem>>;

        #[optional]
        #[method_id(@__retain_semantics Other dragInteraction:sessionForAddingItems:withTouchAtPoint:)]
        unsafe fn dragInteraction_sessionForAddingItems_withTouchAtPoint(
            &self,
            interaction: &UIDragInteraction,
            sessions: &NSArray<ProtocolObject<dyn UIDragSession>>,
            point: CGPoint,
        ) -> Id<NSObject>;

        #[optional]
        #[method(dragInteraction:willAnimateLiftWithAnimator:session:)]
        unsafe fn dragInteraction_willAnimateLiftWithAnimator_session(
            &self,
            interaction: &UIDragInteraction,
            animator: &ProtocolObject<dyn UIDragAnimating>,
            session: &ProtocolObject<dyn UIDragSession>,
        );

        #[optional]
        #[method(dragInteraction:item:willAnimateCancelWithAnimator:)]
        unsafe fn dragInteraction_item_willAnimateCancelWithAnimator(
            &self,
            interaction: &UIDragInteraction,
            item: &UIDragItem,
            animator: &ProtocolObject<dyn UIDragAnimating>,
        );

        #[optional]
        #[method(dragInteraction:sessionWillBegin:)]
        unsafe fn dragInteraction_sessionWillBegin(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
        );

        #[optional]
        #[method(dragInteraction:session:willAddItems:forInteraction:)]
        unsafe fn dragInteraction_session_willAddItems_forInteraction(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
            items: &NSArray<UIDragItem>,
            adding_interaction: &UIDragInteraction,
        );

        #[optional]
        #[method(dragInteraction:sessionDidMove:)]
        unsafe fn dragInteraction_sessionDidMove(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
        );

        #[optional]
        #[method(dragInteraction:session:willEndWithOperation:)]
        unsafe fn dragInteraction_session_willEndWithOperation(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
            operation: UIDropOperation,
        );

        #[optional]
        #[method(dragInteraction:session:didEndWithOperation:)]
        unsafe fn dragInteraction_session_didEndWithOperation(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
            operation: UIDropOperation,
        );

        #[optional]
        #[method(dragInteraction:sessionDidTransferItems:)]
        unsafe fn dragInteraction_sessionDidTransferItems(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
        );

        #[optional]
        #[method_id(@__retain_semantics Other dragInteraction:previewForLiftingItem:session:)]
        unsafe fn dragInteraction_previewForLiftingItem_session(
            &self,
            interaction: &UIDragInteraction,
            item: &UIDragItem,
            session: &ProtocolObject<dyn UIDragSession>,
        ) -> Option<Id<UITargetedDragPreview>>;

        #[optional]
        #[method_id(@__retain_semantics Other dragInteraction:previewForCancellingItem:withDefault:)]
        unsafe fn dragInteraction_previewForCancellingItem_withDefault(
            &self,
            interaction: &UIDragInteraction,
            item: &UIDragItem,
            default_preview: &UITargetedDragPreview,
        ) -> Option<Id<UITargetedDragPreview>>;

        #[optional]
        #[method(dragInteraction:prefersFullSizePreviewsForSession:)]
        unsafe fn dragInteraction_prefersFullSizePreviewsForSession(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
        ) -> bool;

        #[optional]
        #[method(dragInteraction:sessionIsRestrictedToDraggingApplication:)]
        unsafe fn dragInteraction_sessionIsRestrictedToDraggingApplication(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
        ) -> bool;

        #[optional]
        #[method(dragInteraction:sessionAllowsMoveOperation:)]
        unsafe fn dragInteraction_sessionAllowsMoveOperation(
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
    pub(crate) struct UIDropInteraction;

    unsafe impl ClassType for UIDropInteraction {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_methods!(
    unsafe impl UIDropInteraction {
        #[method_id(@__retain_semantics Init initWithDelegate:)]
        pub unsafe fn initWithDelegate(
            this: Option<Allocated<Self>>,
            delegate: &ProtocolObject<dyn UIDropInteractionDelegate>,
        ) -> Id<Self>;
    }
);

extern_protocol!(
    pub unsafe trait UIDropInteractionDelegate: NSObjectProtocol {
        #[optional]
        #[method(dropInteraction:canHandleSession:)]
        unsafe fn dropInteraction_canHandleSession(
            &self,
            interaction: &UIDropInteraction,
            session: &ProtocolObject<dyn UIDropSession>,
        ) -> bool;

        #[optional]
        #[method(dropInteraction:sessionDidEnter:)]
        unsafe fn dropInteraction_sessionDidEnter(
            &self,
            interaction: &UIDropInteraction,
            session: &ProtocolObject<dyn UIDropSession>,
        );

        #[optional]
        #[method_id(@__retain_semantics Other dropInteraction:sessionDidUpdate:)]
        unsafe fn dropInteraction_sessionDidUpdate(
            &self,
            interaction: &UIDropInteraction,
            session: &ProtocolObject<dyn UIDropSession>,
        ) -> Id<UIDropProposal>;

        #[optional]
        #[method(dropInteraction:sessionDidExit:)]
        unsafe fn dropInteraction_sessionDidExit(
            &self,
            interaction: &UIDropInteraction,
            session: &ProtocolObject<dyn UIDropSession>,
        );

        #[optional]
        #[method(dropInteraction:performDrop:)]
        unsafe fn dropInteraction_performDrop(
            &self,
            interaction: &UIDropInteraction,
            session: &ProtocolObject<dyn UIDropSession>,
        );

        #[optional]
        #[method(dropInteraction:concludeDrop:)]
        unsafe fn dropInteraction_concludeDrop(
            &self,
            interaction: &UIDropInteraction,
            session: &ProtocolObject<dyn UIDropSession>,
        );

        #[optional]
        #[method(dropInteraction:sessionDidEnd:)]
        unsafe fn dropInteraction_sessionDidEnd(
            &self,
            interaction: &UIDropInteraction,
            session: &ProtocolObject<dyn UIDropSession>,
        );

        #[optional]
        #[method_id(@__retain_semantics Other dropInteraction:previewForDroppingItem:withDefault:)]
        unsafe fn dropInteraction_previewForDroppingItem_withDefault(
            &self,
            interaction: &UIDropInteraction,
            item: &UIDragItem,
            default_preview: &UITargetedDragPreview,
        ) -> Option<Id<UITargetedDragPreview>>;

        #[optional]
        #[method(dropInteraction:item:willAnimateDropWithAnimator:)]
        fn dropInteraction_item_willAnimateDropWithAnimator(
            &self,
            interaction: &UIDropInteraction,
            item: &UIDragItem,
            animator: &ProtocolObject<dyn UIDragAnimating>,
        );
    }

    unsafe impl ProtocolType for dyn UIDropInteractionDelegate {}
);

extern_protocol!(
    pub unsafe trait UIContextMenuInteractionDelegate: NSObjectProtocol {
        #[method_id(@__retain_semantics Other contextMenuInteraction:configurationForMenuAtLocation:)]
        unsafe fn contextMenuInteraction_configurationForMenuAtLocation(
            &self,
            interaction: &UIContextMenuInteraction,
            location: CGPoint,
        ) -> Option<Id<UIContextMenuConfiguration>>;

        #[optional]
        #[method_id(@__retain_semantics Other contextMenuInteraction:previewForHighlightingMenuWithConfiguration:)]
        unsafe fn contextMenuInteraction_previewForHighlightingMenuWithConfiguration(
            &self,
            interaction: &UIContextMenuInteraction,
            configuration: &UIContextMenuConfiguration,
        ) -> Option<Id<UITargetedPreview>>;

        #[optional]
        #[method(contextMenuInteraction:willPerformPreviewActionForMenuWithConfiguration:animator:)]
        unsafe fn contextMenuInteraction_willPerformPreviewActionForMenuWithConfiguration_animator(
            &self,
            interaction: &UIContextMenuInteraction,
            configuration: &UIContextMenuConfiguration,
            animator: &ProtocolObject<dyn UIContextMenuInteractionAnimating>,
        );

        #[optional]
        #[method(contextMenuInteraction:willDisplayMenuForConfiguration:animator:)]
        unsafe fn contextMenuInteraction_willDisplayMenuForConfiguration_animator(
            &self,
            interaction: &UIContextMenuInteraction,
            configuration: &UIContextMenuConfiguration,
            animator: Option<&ProtocolObject<dyn UIContextMenuInteractionAnimating>>,
        );

        #[optional]
        #[method(contextMenuInteraction:willEndForConfiguration:animator:)]
        unsafe fn contextMenuInteraction_willEndForConfiguration_animator(
            &self,
            interaction: &UIContextMenuInteraction,
            configuration: &UIContextMenuConfiguration,
            animator: Option<&ProtocolObject<dyn UIContextMenuInteractionAnimating>>,
        );
    }
    unsafe impl ProtocolType for dyn UIContextMenuInteractionDelegate {}
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIContextMenuInteraction;

    unsafe impl ClassType for UIContextMenuInteraction {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_methods!(
    unsafe impl UIContextMenuInteraction {
        #[method_id(@__retain_semantics Init initWithDelegate:)]
        pub unsafe fn initWithDelegate(
            this: Option<Allocated<Self>>,
            delegate: &ProtocolObject<dyn UIContextMenuInteractionDelegate>,
        ) -> Id<Self>;
    }
);

pub type UIMenuElementState = NSInteger;
pub const UIMenuElementStateOff: UIMenuElementState = 0;
pub const UIMenuElementStateOn: UIMenuElementState = 1;
pub const UIMenuElementStateMixed: UIMenuElementState = 2;

pub type UIMenuElementAttributes = NSUInteger;
pub const UIMenuElementAttributesDisabled: UIMenuElementAttributes = 1 << 0;
pub const UIMenuElementAttributesDestructive: UIMenuElementAttributes = 1 << 1;
pub const UIMenuElementAttributesHidden: UIMenuElementAttributes = 1 << 2;

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIMenuElement;

    unsafe impl ClassType for UIMenuElement {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIAction;

    unsafe impl ClassType for UIAction {
        type Super = UIMenuElement;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_methods!(
    unsafe impl UIAction {
        #[method_id(@__retain_semantics Other actionWithTitle:image:identifier:handler:)]
        pub unsafe fn actionWithTitle_image_identifier_handler(
            title: &NSString,
            image: Option<&UIImage>,
            identifier: Option<&NSString>,
            handler: &Block<(&UIAction,), ()>,
        ) -> Id<Self>;

        #[method(setAttributes:)]
        pub unsafe fn setAttributes(&self, attributes: UIMenuElementAttributes);

        #[method(attributes)]
        pub unsafe fn attributes(&self) -> UIMenuElementAttributes;

        #[method(setState:)]
        pub unsafe fn setState(&self, state: UIMenuElementState);

        #[method(state)]
        pub unsafe fn state(&self) -> UIMenuElementState;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIMenu;

    unsafe impl ClassType for UIMenu {
        type Super = UIMenuElement;
        type Mutability = mutability::InteriorMutable;
    }
);

pub type UIMenuOptions = NSUInteger;
pub const UIMenuOptionsDisplayInline: UIMenuOptions = 1 << 0;
pub const UIMenuOptionsDestructive: UIMenuOptions = 1 << 1;
pub const UIMenuOptionsSingleSelection: UIMenuOptions = 1 << 5;
pub const UIMenuOptionsDisplayAsPalette: UIMenuOptions = 1 << 7;

extern_methods!(
    unsafe impl UIMenu {
        #[method_id(@__retain_semantics Other menuWithTitle:image:identifier:options:children:)]
        pub unsafe fn menuWithTitle_image_identifier_options_children(
            title: &NSString,
            image: Option<&UIImage>,
            identifier: Option<&NSString>,
            options: UIMenuOptions,
            children: &NSArray<UIMenuElement>,
        ) -> Id<Self>;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIDeferredMenuElement;

    unsafe impl ClassType for UIDeferredMenuElement {
        type Super = UIMenuElement;
        type Mutability = mutability::InteriorMutable;
    }
);

pub type UIDeferredMenuElementCompletionBlock = Block<(NonNull<NSArray<UIMenuElement>>,), ()>;

extern_methods!(
    unsafe impl UIDeferredMenuElement {
        #[method_id(@__retain_semantics Other elementWithProvider:)]
        pub unsafe fn elementWithProvider(
            provider: &Block<(NonNull<UIDeferredMenuElementCompletionBlock>,), ()>,
        ) -> Id<Self>;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct UIContextMenuConfiguration;

    unsafe impl ClassType for UIContextMenuConfiguration {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

pub type UIContextMenuPreviewProvider = Block<(), *mut UIViewController>;
pub type UIContextMenuActionProvider = Block<(NonNull<NSArray<UIMenuElement>>,), *mut UIMenu>;

extern_methods!(
    unsafe impl UIContextMenuConfiguration {
        #[method_id(@__retain_semantics Other configurationWithIdentifier:previewProvider:actionProvider:)]
        pub unsafe fn configurationWithIdentifier_previewProvider_actionProvider(
            identifier: Option<&NSString>,
            preview_provider: Option<&UIContextMenuPreviewProvider>,
            action_provider: Option<&UIContextMenuActionProvider>,
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
        pub unsafe fn clearColor() -> Id<Self>;

        #[method_id(@__retain_semantics Other whiteColor)]
        pub unsafe fn whiteColor() -> Id<Self>;
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
        pub unsafe fn bezierPath() -> Id<Self>;

        #[method_id(@__retain_semantics Other bezierPathWithRect:)]
        pub unsafe fn bezierPathWithRect(rect: CGRect) -> Id<Self>;

        #[method(appendPath:)]
        pub unsafe fn appendPath(&self, path: &UIBezierPath);
    }
);
