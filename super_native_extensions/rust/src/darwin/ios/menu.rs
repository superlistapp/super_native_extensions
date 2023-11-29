use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt::Formatter,
    ptr::NonNull,
    rc::{Rc, Weak},
    time::Duration,
};

use icrate::{
    block2::{ConcreteBlock, RcBlock},
    Foundation::{CGPoint, CGRect, CGSize, NSArray, NSString},
};
use irondash_engine_context::EngineContext;
use irondash_message_channel::{IsolateId, Late};
use irondash_run_loop::{platform::PollSession, spawn, RunLoop};

use objc2::{
    declare::{Ivar, IvarDrop},
    declare_class, msg_send_id, mutability,
    rc::Id,
    runtime::{NSObject, NSObjectProtocol, ProtocolObject},
    ClassType,
};

use crate::{
    api_model::{
        ImageData, Menu, MenuActionState, MenuConfiguration, MenuElement, MenuImage,
        ShowContextMenuRequest, ShowContextMenuResponse,
    },
    error::{NativeExtensionsError, NativeExtensionsResult},
    menu_manager::{PlatformMenuContextDelegate, PlatformMenuContextId, PlatformMenuDelegate},
    platform_impl::platform::{
        common::UnsafeMutRef,
        os::{
            uikit::{UIMenu, UIMenuOptionsDisplayInline},
            util::{image_view_from_data, IgnoreInteractionEvents},
        },
    },
    value_promise::PromiseResult,
};

use super::{
    alpha_to_path::bezier_path_for_alpha,
    uikit::{
        UIAction, UIActivityIndicatorView, UIActivityIndicatorViewStyleMedium, UIColor,
        UIContextMenuConfiguration, UIContextMenuInteraction, UIContextMenuInteractionAnimating,
        UIContextMenuInteractionDelegate, UIDeferredMenuElement,
        UIDeferredMenuElementCompletionBlock, UIImage, UIImageView, UIMenuElement,
        UIMenuElementAttributes, UIMenuElementAttributesDestructive,
        UIMenuElementAttributesDisabled, UIMenuElementState, UIMenuElementStateMixed,
        UIMenuElementStateOff, UIMenuElementStateOn, UIPreviewParameters, UIPreviewTarget,
        UITargetedPreview, UIView, UIViewController,
    },
    util::image_from_image_data,
};

pub struct PlatformMenuContext {
    id: PlatformMenuContextId,
    weak_self: Late<Weak<Self>>,
    view: Id<UIView>,
    delegate: Weak<dyn PlatformMenuContextDelegate>,
    interaction: Late<Id<UIContextMenuInteraction>>,
    interaction_delegate: Late<Id<SNEMenuContext>>,
    sessions: RefCell<HashMap<usize, MenuSession>>,
}

pub struct PlatformMenu {
    ui_menu: Id<UIMenu>,
    item_selected: Rc<Cell<bool>>,
}

impl std::fmt::Debug for PlatformMenu {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformMenu").finish()
    }
}

impl PlatformMenu {
    pub fn new(
        isolate: IsolateId,
        delegate: Weak<dyn PlatformMenuDelegate>,
        menu: Menu,
    ) -> NativeExtensionsResult<Rc<Self>> {
        let item_selected = Rc::new(Cell::new(false));
        let res = Self {
            item_selected: item_selected.clone(),
            ui_menu: unsafe {
                Id::cast(Self::convert_menu(
                    MenuElement::Menu(menu),
                    isolate,
                    &delegate,
                    item_selected,
                )?)
            },
        };
        Ok(Rc::new(res))
    }

    unsafe fn convert_string(str: &Option<String>) -> Option<Id<NSString>> {
        str.as_ref().map(|str| NSString::from_str(str))
    }

    unsafe fn convert_image(image: &Option<MenuImage>) -> Option<Id<UIImage>> {
        match image {
            Some(MenuImage::Image { data }) => Some(image_from_image_data(data.clone())),
            Some(MenuImage::System { name }) => {
                let name = NSString::from_str(name);
                UIImage::systemImageNamed(&name)
            }
            None => None,
        }
    }

    unsafe fn convert_elements(
        elements: Vec<MenuElement>,
        isolate_id: IsolateId,
        delegate: &Weak<dyn PlatformMenuDelegate>,
        item_selected: Rc<Cell<bool>>,
    ) -> NativeExtensionsResult<Vec<Id<UIMenuElement>>> {
        let mut res = Vec::new();

        struct InlineSection {
            title: Id<NSString>,
            elements: Vec<Id<UIMenuElement>>,
        }

        let mut inline_section = None::<InlineSection>;

        unsafe fn finish_inline_section(
            inline_section: Option<InlineSection>,
        ) -> Vec<Id<UIMenuElement>> {
            if let Some(inline_section) = inline_section {
                let elements = NSArray::from_vec(inline_section.elements);
                let res = UIMenu::menuWithTitle_image_identifier_options_children(
                    &inline_section.title,
                    None,
                    None,
                    UIMenuOptionsDisplayInline,
                    &elements,
                );
                vec![Id::into_super(res)]
            } else {
                Vec::new()
            }
        }

        for element in elements {
            match element {
                MenuElement::Separator(separator) => {
                    res.append(&mut finish_inline_section(inline_section));
                    inline_section = Some(InlineSection {
                        title: Self::convert_string(&separator.title).unwrap_or_default(),
                        elements: Vec::new(),
                    });
                }
                element => {
                    let converted =
                        Self::convert_menu(element, isolate_id, delegate, item_selected.clone())?;
                    if let Some(inline_section) = inline_section.as_mut() {
                        inline_section.elements.push(converted);
                    } else {
                        res.push(converted);
                    }
                }
            }
        }

        res.append(&mut finish_inline_section(inline_section));

        Ok(res)
    }

    unsafe fn convert_menu(
        menu: MenuElement,
        isolate_id: IsolateId,
        delegate: &Weak<dyn PlatformMenuDelegate>,
        item_selected: Rc<Cell<bool>>,
    ) -> NativeExtensionsResult<Id<UIMenuElement>> {
        match menu {
            MenuElement::Action(action) => {
                let unique_id = action.unique_id;
                let delegate = delegate.clone();
                let handler = ConcreteBlock::new(move |_| {
                    item_selected.set(true);
                    if let Some(delegate) = delegate.upgrade() {
                        delegate.on_action(isolate_id, unique_id);
                    }
                });
                let handler = handler.copy();
                let res = UIAction::actionWithTitle_image_identifier_handler(
                    &Self::convert_string(&action.title).unwrap_or_default(),
                    Self::convert_image(&action.image).as_deref(),
                    Self::convert_string(&action.identifier).as_deref(),
                    &handler,
                );
                let mut options: UIMenuElementAttributes = 0;
                if action.attributes.disabled {
                    options |= UIMenuElementAttributesDisabled;
                }
                if action.attributes.destructive {
                    options |= UIMenuElementAttributesDestructive;
                }
                res.setAttributes(options);

                let state: UIMenuElementState = match action.state {
                    MenuActionState::None => UIMenuElementStateOff,
                    MenuActionState::CheckOff => UIMenuElementStateOff,
                    MenuActionState::RadioOff => UIMenuElementStateOff,
                    MenuActionState::CheckOn => UIMenuElementStateOn,
                    MenuActionState::RadioOn => UIMenuElementStateOn,
                    MenuActionState::CheckMixed => UIMenuElementStateMixed,
                };
                res.setState(state);

                Ok(Id::into_super(res))
            }
            MenuElement::Menu(menu) => {
                let children =
                    Self::convert_elements(menu.children, isolate_id, delegate, item_selected)?;
                let children = NSArray::from_vec(children);
                let menu = UIMenu::menuWithTitle_image_identifier_options_children(
                    &Self::convert_string(&menu.title).unwrap_or_default(),
                    Self::convert_image(&menu.image).as_deref(),
                    Self::convert_string(&menu.identifier).as_deref(),
                    0,
                    &children,
                );
                Ok(Id::into_super(menu))
            }
            MenuElement::Deferred(deferred) => {
                let delegate = delegate.clone();
                let provider = ConcreteBlock::new(
                    move |completion_block: NonNull<UIDeferredMenuElementCompletionBlock>| {
                        let delegate = delegate.clone();
                        let item_selected = item_selected.clone();
                        let completion_block = unsafe { RcBlock::copy(completion_block.as_ptr()) };
                        spawn(async move {
                            if let Some(delegate) = delegate.upgrade() {
                                let menu = delegate
                                    .get_deferred_menu(isolate_id, deferred.unique_id)
                                    .await;

                                let array = match menu {
                                    Ok(elements) => {
                                        let elements = Self::convert_elements(
                                            elements,
                                            isolate_id,
                                            &Rc::downgrade(&delegate),
                                            item_selected,
                                        );
                                        match elements {
                                            Ok(elements) => Some(NSArray::from_vec(elements)),
                                            Err(_) => None,
                                        }
                                    }
                                    Err(_) => None,
                                };
                                let array = array.unwrap_or_default();
                                let array =
                                    unsafe { NonNull::new_unchecked(Id::as_ptr(&array) as *mut _) };
                                completion_block.call((array,));
                            }
                        });
                    },
                );
                let provider = provider.copy();
                let res = UIDeferredMenuElement::elementWithProvider(&provider);
                Ok(Id::into_super(res))
            }
            MenuElement::Separator(_separator) => {
                panic!("Separator should be converted to inline section")
            }
        }
    }
}

struct MenuSession {
    _id: Id<UIContextMenuConfiguration>,
    view_container: Id<UIView>,
    view_controller: Id<UIViewController>,
    configuration: MenuConfiguration,
}

impl MenuSession {
    pub fn get_id(configuration: &UIContextMenuConfiguration) -> usize {
        configuration as *const _ as usize
    }

    fn update_preview_image(&self, image: ImageData) {
        let preview_view = image_view_from_data(image);
        unsafe {
            let view = self.view_controller.view().unwrap();

            let sub_views = view.subviews();
            // There is no activity indicator subview, meaning the preview block has not been called yet.
            // in which case just set the image and be done with it. the preview_provider will check
            // if the view is an image view and if so, will leave it alone.
            if sub_views.count() == 0 {
                self.view_controller.setView(Some(&preview_view));
                return;
            }
            let prev_subview = sub_views.objectAtIndex(0);
            view.addSubview(&preview_view);
            preview_view.setAlpha(0.0);

            let prev_copy = prev_subview.clone();
            let animation = ConcreteBlock::new(move || {
                preview_view.setAlpha(1.0);
                prev_copy.setAlpha(0.0);
            });
            let animation = animation.copy();
            let completion = ConcreteBlock::new(move |_| {
                prev_subview.removeFromSuperview();
            });
            let completion = completion.copy();
            UIView::animateWithDuration_animations_completion(0.25, &animation, Some(&completion));
        }
    }
}

impl PlatformMenuContext {
    pub fn new(
        id: PlatformMenuContextId,
        engine_handle: i64,
        delegate: Weak<dyn PlatformMenuContextDelegate>,
    ) -> NativeExtensionsResult<Self> {
        let view = EngineContext::get()?.get_flutter_view(engine_handle)?;

        Ok(Self {
            id,
            weak_self: Late::new(),
            view: unsafe { Id::cast(view) },
            delegate,
            interaction: Late::new(),
            interaction_delegate: Late::new(),
            sessions: RefCell::new(HashMap::new()),
        })
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self.clone());
        let delegate = SNEMenuContext::new(weak_self);
        self.interaction_delegate.set(delegate.retain());
        let interaction = unsafe {
            UIContextMenuInteraction::initWithDelegate(
                UIContextMenuInteraction::alloc(),
                &Id::cast(delegate),
            )
        };
        unsafe { self.view.addInteraction(&interaction) };
    }

    pub fn menu_active(&self) -> bool {
        !self.sessions.borrow().is_empty()
    }

    pub fn update_preview_image(
        &self,
        configuration_id: i64,
        image_data: ImageData,
    ) -> NativeExtensionsResult<()> {
        let sessions = self.sessions.borrow();
        let session = sessions
            .values()
            .find(|s| s.configuration.configuration_id == configuration_id);
        if let Some(session) = session {
            session.update_preview_image(image_data);
        }
        Ok(())
    }

    pub async fn show_context_menu(
        &self,
        _request: ShowContextMenuRequest,
    ) -> NativeExtensionsResult<ShowContextMenuResponse> {
        Err(NativeExtensionsError::UnsupportedOperation)
    }

    fn _configuration_for_menu_at_location(
        &self,
        _interaction: &UIContextMenuInteraction,
        menu_configuration: MenuConfiguration,
    ) -> Id<UIContextMenuConfiguration> {
        let view_controller = unsafe { UIViewController::init(UIViewController::alloc()) };

        let menu = menu_configuration.menu.as_ref().unwrap().ui_menu.clone();
        let configuration = unsafe {
            let menu = Rc::new(menu); // Id is not Clone
            let action_provider =
                ConcreteBlock::new(move |_suggested| Id::autorelease_return(menu.retain()));
            let action_provider = action_provider.copy();

            let preview_provider = match (
                menu_configuration.preview_image.as_ref(),
                menu_configuration.preview_size.as_ref(),
            ) {
                (Some(preview_image), None) => {
                    let preview_view = image_view_from_data(preview_image.clone());
                    let size = CGSize {
                        width: preview_image.point_width(),
                        height: preview_image.point_height(),
                    };
                    let controller = view_controller.retain();
                    let preview_provider = ConcreteBlock::new(move || {
                        controller.setView(Some(&preview_view));
                        controller.setPreferredContentSize(size);
                        Id::autorelease_return(controller.retain())
                    });
                    let preview_provider = preview_provider.copy();
                    Some(preview_provider)
                }
                (None, Some(size)) => {
                    let controller = view_controller.retain();
                    let size: CGSize = size.clone().into();
                    let preview_provider = ConcreteBlock::new(move || {
                        controller.setPreferredContentSize(size);
                        let view = controller.view();
                        if let Some(view) = view {
                            let is_image_view = view.is_kind_of::<UIImageView>();
                            if is_image_view {
                                return Id::autorelease_return(controller.retain());
                            }
                        }
                        let view = UIView::initWithFrame(UIView::alloc(), CGRect::ZERO);
                        let activity_indicator = {
                            let res = UIActivityIndicatorView::initWithActivityIndicatorStyle(
                                UIActivityIndicatorView::alloc(),
                                UIActivityIndicatorViewStyleMedium,
                            );
                            res.startAnimating();
                            res.setCenter(CGPoint {
                                x: size.width / 2.0,
                                y: size.height / 2.0,
                            });
                            res.setColor(Some(&UIColor::whiteColor()));
                            res
                        };
                        view.addSubview(&activity_indicator);
                        controller.setView(Some(&view));
                        Id::autorelease_return(controller.retain())
                    });
                    let preview_provider = preview_provider.copy();
                    Some(preview_provider)
                }
                _ => None,
            };

            UIContextMenuConfiguration::configurationWithIdentifier_previewProvider_actionProvider(
                None,
                preview_provider.as_deref(),
                Some(&action_provider),
            )
        };
        let view_container = unsafe {
            let bounds = self.view.bounds();
            let container = UIView::initWithFrame(UIView::alloc(), bounds);
            container.setUserInteractionEnabled(false);
            self.view.addSubview(&container);
            container
        };

        let session = MenuSession {
            _id: configuration.clone(),
            view_container,
            configuration: menu_configuration,
            view_controller,
        };
        self.sessions
            .borrow_mut()
            .insert(MenuSession::get_id(&configuration), session);
        configuration
    }

    fn configuration_for_menu_at_location(
        &self,
        interaction: &UIContextMenuInteraction,
        location: CGPoint,
    ) -> Option<Id<UIContextMenuConfiguration>> {
        if let Some(delegate) = self.delegate.upgrade() {
            let configuration_promise =
                delegate.get_menu_configuration_for_location(self.id, location.into());
            let mut poll_session = PollSession::new();

            // See drag.rs items_for_beginning
            let _ignore_events = IgnoreInteractionEvents::new();
            loop {
                if let Some(configuration) = configuration_promise.try_take() {
                    match configuration {
                        PromiseResult::Ok { value } => {
                            return Some(
                                self._configuration_for_menu_at_location(interaction, value),
                            );
                        }
                        PromiseResult::Cancelled => {
                            return None;
                        }
                    }
                }
                RunLoop::current()
                    .platform_run_loop
                    .poll_once(&mut poll_session);
            }
        } else {
            None
        }
    }

    fn preview_for_highlighting_menu_with_configuration(
        &self,
        _interaction: &UIContextMenuInteraction,
        configuration: &UIContextMenuConfiguration,
    ) -> Option<Id<UITargetedPreview>> {
        let sessions = self.sessions.borrow();
        let session = sessions.get(&MenuSession::get_id(configuration));
        match session {
            Some(session) => unsafe {
                let image = &session.configuration.lift_image;
                let lift_image = image_view_from_data(image.image_data.clone());
                let frame: CGRect = image.rect.translated(-100000.0, -100000.0).into();
                lift_image.setFrame(frame);
                session.view_container.addSubview(&lift_image);
                let parameters = UIPreviewParameters::init(UIPreviewParameters::alloc());
                let shadow_path = bezier_path_for_alpha(&image.image_data);
                parameters.setShadowPath(Some(&shadow_path));

                // This is a workaround around the fact that after transitioning to menu
                // the shadow path is not clipped together with the view. So we set it to
                // nil which will cause UIKit to draw round corner shadow matching the view
                // clip rect. The duration doesn't seem to matter, it just need to be called
                // before the transition so that UIKit picks it out.
                let parameters_clone = parameters.clone();
                RunLoop::current()
                    .schedule(Duration::from_millis(10), move || {
                        parameters_clone.setShadowPath(None);
                    })
                    .detach();

                parameters.setBackgroundColor(Some(&UIColor::clearColor()));

                let center: CGPoint = image.rect.center().into();
                let target = UIPreviewTarget::initWithContainer_center(
                    UIPreviewTarget::alloc(),
                    &session.view_container,
                    center,
                );

                let preview = UITargetedPreview::initWithView_parameters_target(
                    UITargetedPreview::alloc(),
                    &lift_image,
                    &parameters,
                    &target,
                );
                Some(preview)
            },
            _ => None,
        }
    }

    fn interaction_will_display_menu_for_configuration(
        &self,
        _interaction: &UIContextMenuInteraction,
        configuration: &UIContextMenuConfiguration,
        _animator: Option<&ProtocolObject<dyn UIContextMenuInteractionAnimating>>,
    ) {
        let sessions = self.sessions.borrow();
        let session = sessions.get(&MenuSession::get_id(configuration));
        let delegate = self.delegate.upgrade();
        if let (Some(session), Some(delegate)) = (session, delegate) {
            delegate.on_show_menu(self.id, session.configuration.configuration_id);
        }
    }

    fn interaction_will_perform_preview_action_for_menu_with_configuration(
        &self,
        _interaction: &UIContextMenuInteraction,
        configuration: &UIContextMenuConfiguration,
        _animator: &ProtocolObject<dyn UIContextMenuInteractionAnimating>,
    ) {
        let sessions = self.sessions.borrow();
        let session = sessions.get(&MenuSession::get_id(configuration));
        let delegate = self.delegate.upgrade();
        if let (Some(session), Some(delegate)) = (session, delegate) {
            delegate.on_preview_action(self.id, session.configuration.configuration_id);
        }
    }

    fn interaction_will_end_for_configuration(
        &self,
        _interaction: &UIContextMenuInteraction,
        configuration: &UIContextMenuConfiguration,
        animator: Option<&ProtocolObject<dyn UIContextMenuInteractionAnimating>>,
    ) {
        let session = self
            .sessions
            .borrow_mut()
            .remove(&MenuSession::get_id(configuration));
        if let Some(session) = session {
            unsafe {
                let container = session.view_container.clone();
                let animation = ConcreteBlock::new(move || {
                    container.setAlpha(0.0);
                });
                let animation = animation.copy();
                if let Some(animator) = animator {
                    animator.addAnimations(&animation);
                }

                let completion = ConcreteBlock::new(move || {
                    session.view_container.removeFromSuperview();
                });
                let completion = completion.copy();
                if let Some(animator) = animator {
                    animator.addCompletion(&completion);
                } else {
                    completion.call(())
                }
            }
            if let Some(delegate) = self.delegate.upgrade() {
                let item_selected = session
                    .configuration
                    .menu
                    .map(|m| m.item_selected.get())
                    .unwrap_or(false);
                delegate.on_hide_menu(
                    self.id,
                    session.configuration.configuration_id,
                    item_selected,
                );
            }
        }
    }
}

impl Drop for PlatformMenuContext {
    fn drop(&mut self) {
        unsafe { self.view.removeInteraction(&self.interaction) };
    }
}

pub struct Inner {
    context: Weak<PlatformMenuContext>,
}

impl Inner {
    fn with_state<F, FR, R>(&self, callback: F, default: FR) -> R
    where
        F: FnOnce(Rc<PlatformMenuContext>) -> R,
        FR: FnOnce() -> R,
    {
        let context = self.context.upgrade();
        match context {
            Some(context) => callback(context),
            None => default(),
        }
    }
}

declare_class!(
    struct SNEMenuContext {
        context: IvarDrop<Box<Inner>, "_context">,
    }

    mod ivars;

    unsafe impl ClassType for SNEMenuContext {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
        const NAME: &'static str = "SNEMenuContext";
    }

    unsafe impl NSObjectProtocol for SNEMenuContext {}

    #[allow(non_snake_case)]
    unsafe impl UIContextMenuInteractionDelegate for SNEMenuContext {
        #[method_id(contextMenuInteraction:configurationForMenuAtLocation:)]
        fn contextMenuInteraction_configurationForMenuAtLocation(
            &self,
            interaction: &UIContextMenuInteraction,
            location: CGPoint,
        ) -> Option<Id<UIContextMenuConfiguration>> {
            self.context.with_state(
                |state| state.configuration_for_menu_at_location(interaction, location),
                || None,
            )
        }

        #[method_id(contextMenuInteraction:previewForHighlightingMenuWithConfiguration:)]
        fn contextMenuInteraction_previewForHighlightingMenuWithConfiguration(
            &self,
            interaction: &UIContextMenuInteraction,
            configuration: &UIContextMenuConfiguration,
        ) -> Option<Id<UITargetedPreview>> {
            self.context.with_state(
                |state| {
                    state.preview_for_highlighting_menu_with_configuration(
                        interaction,
                        configuration,
                    )
                },
                || None,
            )
        }

        #[method(contextMenuInteraction:willPerformPreviewActionForMenuWithConfiguration:animator:)]
        fn contextMenuInteraction_willPerformPreviewActionForMenuWithConfiguration_animator(
            &self,
            interaction: &UIContextMenuInteraction,
            configuration: &UIContextMenuConfiguration,
            animator: &ProtocolObject<dyn UIContextMenuInteractionAnimating>,
        ) {
            self.context.with_state(
                |state| {
                    state.interaction_will_perform_preview_action_for_menu_with_configuration(
                        interaction,
                        configuration,
                        animator,
                    )
                },
                || {},
            );
        }

        #[method(contextMenuInteraction:willDisplayMenuForConfiguration:animator:)]
        fn contextMenuInteraction_willDisplayMenuForConfiguration_animator(
            &self,
            interaction: &UIContextMenuInteraction,
            configuration: &UIContextMenuConfiguration,
            animator: Option<&ProtocolObject<dyn UIContextMenuInteractionAnimating>>,
        ) {
            self.context.with_state(
                |state| {
                    state.interaction_will_display_menu_for_configuration(
                        interaction,
                        configuration,
                        animator,
                    )
                },
                || {},
            );
        }

        #[method(contextMenuInteraction:willEndForConfiguration:animator:)]
        fn contextMenuInteraction_willEndForConfiguration_animator(
            &self,
            interaction: &UIContextMenuInteraction,
            configuration: &UIContextMenuConfiguration,
            animator: Option<&ProtocolObject<dyn UIContextMenuInteractionAnimating>>,
        ) {
            self.context.with_state(
                |state| {
                    state.interaction_will_end_for_configuration(
                        interaction,
                        configuration,
                        animator,
                    )
                },
                || {},
            );
        }
    }
);

impl SNEMenuContext {
    fn new(context: Weak<PlatformMenuContext>) -> Id<Self> {
        let this: Id<Self> = unsafe { msg_send_id![Self::alloc(), init] };
        unsafe {
            this.unsafe_mut_ref(|this| {
                Ivar::write(&mut this.context, Box::new(Inner { context }));
            });
        }
        this
    }
}
