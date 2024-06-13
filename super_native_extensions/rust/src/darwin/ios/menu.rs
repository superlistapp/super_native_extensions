use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt::Formatter,
    ops::Deref,
    ptr::NonNull,
    rc::{Rc, Weak},
    time::Duration,
};

use block2::{Block, RcBlock};
use irondash_engine_context::EngineContext;
use irondash_message_channel::{IsolateId, Late};
use irondash_run_loop::{platform::PollSession, spawn, RunLoop};
use objc2_foundation::{CGPoint, CGRect, CGSize, MainThreadMarker, NSArray, NSString};

use objc2::{
    declare_class, msg_send_id, mutability,
    rc::{Id, Retained},
    runtime::{NSObject, NSObjectProtocol, ProtocolObject},
    sel, ClassType, DeclaredClass,
};
use objc2_ui_kit::{
    UIAction, UIActivityIndicatorView, UIActivityIndicatorViewStyle, UIColor,
    UIContextMenuConfiguration, UIContextMenuInteraction, UIContextMenuInteractionAnimating,
    UIContextMenuInteractionDelegate, UIDeferredMenuElement, UIGestureRecognizerState, UIImage,
    UIImageView, UIMenu, UIMenuElement, UIMenuElementAttributes, UIMenuElementState, UIMenuOptions,
    UIPanGestureRecognizer, UIPreviewParameters, UIPreviewTarget, UITargetedPreview, UIView,
    UIViewAnimationOptions, UIViewController,
};

use crate::{
    api_model::{
        ImageData, Menu, MenuActionState, MenuConfiguration, MenuElement, MenuImage,
        ShowContextMenuRequest, ShowContextMenuResponse,
    },
    error::{NativeExtensionsError, NativeExtensionsResult},
    menu_manager::{PlatformMenuContextDelegate, PlatformMenuContextId, PlatformMenuDelegate},
    platform_impl::platform::os::util::{image_view_from_data, IgnoreInteractionEvents},
    value_promise::PromiseResult,
};

use super::{alpha_to_path::bezier_path_for_alpha, util::image_from_image_data};

pub struct PlatformMenuContext {
    id: PlatformMenuContextId,
    weak_self: Late<Weak<Self>>,
    view: Id<UIView>,
    delegate: Weak<dyn PlatformMenuContextDelegate>,
    interaction: Late<Id<UIContextMenuInteraction>>,
    interaction_delegate: Late<Id<SNEMenuContext>>,
    sessions: RefCell<HashMap<usize, MenuSession>>,
    fading_containers: RefCell<Vec<Retained<UIView>>>,
    mtm: MainThreadMarker,
}

pub struct PlatformMenu {
    ui_menu: Id<UIMenu>,
    item_selected: Rc<Cell<bool>>,
    mtm: MainThreadMarker,
}

impl std::fmt::Debug for PlatformMenu {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformMenu").finish()
    }
}

pub type UIDeferredMenuElementCompletionBlock = Block<dyn Fn(NonNull<NSArray<UIMenuElement>>)>;

impl PlatformMenu {
    pub fn new(
        isolate: IsolateId,
        delegate: Weak<dyn PlatformMenuDelegate>,
        menu: Menu,
    ) -> NativeExtensionsResult<Rc<Self>> {
        let mtm = MainThreadMarker::new().unwrap();
        let item_selected = Rc::new(Cell::new(false));
        let res = Self {
            item_selected: item_selected.clone(),
            ui_menu: unsafe {
                Id::cast(Self::convert_menu(
                    MenuElement::Menu(menu),
                    isolate,
                    &delegate,
                    item_selected,
                    mtm,
                )?)
            },
            mtm,
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
        mtm: MainThreadMarker,
    ) -> NativeExtensionsResult<Vec<Id<UIMenuElement>>> {
        let mut res = Vec::new();

        struct InlineSection {
            title: Id<NSString>,
            elements: Vec<Id<UIMenuElement>>,
        }

        let mut inline_section = None::<InlineSection>;

        unsafe fn finish_inline_section(
            inline_section: Option<InlineSection>,
            mtm: MainThreadMarker,
        ) -> Vec<Id<UIMenuElement>> {
            if let Some(inline_section) = inline_section {
                let elements = NSArray::from_vec(inline_section.elements);
                let res = UIMenu::menuWithTitle_image_identifier_options_children(
                    &inline_section.title,
                    None,
                    None,
                    UIMenuOptions::DisplayInline,
                    &elements,
                    mtm,
                );
                vec![Id::into_super(res)]
            } else {
                Vec::new()
            }
        }

        for element in elements {
            match element {
                MenuElement::Separator(separator) => {
                    res.append(&mut finish_inline_section(inline_section, mtm));
                    inline_section = Some(InlineSection {
                        title: Self::convert_string(&separator.title).unwrap_or_default(),
                        elements: Vec::new(),
                    });
                }
                element => {
                    let converted = Self::convert_menu(
                        element,
                        isolate_id,
                        delegate,
                        item_selected.clone(),
                        mtm,
                    )?;
                    if let Some(inline_section) = inline_section.as_mut() {
                        inline_section.elements.push(converted);
                    } else {
                        res.push(converted);
                    }
                }
            }
        }

        res.append(&mut finish_inline_section(inline_section, mtm));

        Ok(res)
    }

    unsafe fn convert_menu(
        menu: MenuElement,
        isolate_id: IsolateId,
        delegate: &Weak<dyn PlatformMenuDelegate>,
        item_selected: Rc<Cell<bool>>,
        mtm: MainThreadMarker,
    ) -> NativeExtensionsResult<Id<UIMenuElement>> {
        match menu {
            MenuElement::Action(action) => {
                let unique_id = action.unique_id;
                let delegate = delegate.clone();
                let handler = RcBlock::new(move |_: NonNull<UIAction>| {
                    item_selected.set(true);
                    if let Some(delegate) = delegate.upgrade() {
                        delegate.on_action(isolate_id, unique_id);
                    }
                });

                let res = UIAction::actionWithTitle_image_identifier_handler(
                    &Self::convert_string(&action.title).unwrap_or_default(),
                    Self::convert_image(&action.image).as_deref(),
                    Self::convert_string(&action.identifier).as_deref(),
                    handler.deref() as *const _ as *mut _,
                    mtm,
                );
                let mut options = UIMenuElementAttributes::empty();
                options.set(
                    UIMenuElementAttributes::Disabled,
                    action.attributes.disabled,
                );
                options.set(
                    UIMenuElementAttributes::Destructive,
                    action.attributes.destructive,
                );

                res.setAttributes(options);

                let state: UIMenuElementState = match action.state {
                    MenuActionState::None => UIMenuElementState::Off,
                    MenuActionState::CheckOff => UIMenuElementState::Off,
                    MenuActionState::RadioOff => UIMenuElementState::Off,
                    MenuActionState::CheckOn => UIMenuElementState::On,
                    MenuActionState::RadioOn => UIMenuElementState::On,
                    MenuActionState::CheckMixed => UIMenuElementState::Mixed,
                };
                res.setState(state);

                Ok(Id::into_super(res))
            }
            MenuElement::Menu(menu) => {
                let children = Self::convert_elements(
                    menu.children,
                    isolate_id,
                    delegate,
                    item_selected,
                    mtm,
                )?;
                let children = NSArray::from_vec(children);
                let menu = UIMenu::menuWithTitle_image_identifier_options_children(
                    &Self::convert_string(&menu.title).unwrap_or_default(),
                    Self::convert_image(&menu.image).as_deref(),
                    Self::convert_string(&menu.identifier).as_deref(),
                    UIMenuOptions::empty(),
                    &children,
                    mtm,
                );
                Ok(Id::into_super(menu))
            }
            MenuElement::Deferred(deferred) => {
                let delegate = delegate.clone();
                let provider = RcBlock::new(
                    move |completion_block: NonNull<UIDeferredMenuElementCompletionBlock>| {
                        let delegate = delegate.clone();
                        let item_selected = item_selected.clone();
                        let completion_block =
                            unsafe { RcBlock::copy(completion_block.as_ptr()).unwrap() };
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
                                            mtm,
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

                let res = UIDeferredMenuElement::elementWithProvider(&provider, mtm);
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
    mtm: MainThreadMarker,
}

impl MenuSession {
    pub fn get_id(configuration: &UIContextMenuConfiguration) -> usize {
        configuration as *const _ as usize
    }

    fn update_preview_image(&self, image: ImageData) {
        let preview_view = image_view_from_data(image, self.mtm);
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
            let animation = RcBlock::new(move || {
                preview_view.setAlpha(1.0);
                prev_copy.setAlpha(0.0);
            });
            let completion = RcBlock::new(move |_| {
                prev_subview.removeFromSuperview();
            });
            UIView::animateWithDuration_animations_completion(
                0.25,
                &animation,
                Some(&completion),
                self.mtm,
            );
        }
    }
}

impl PlatformMenuContext {
    pub fn new(
        id: PlatformMenuContextId,
        engine_handle: i64,
        delegate: Weak<dyn PlatformMenuContextDelegate>,
    ) -> NativeExtensionsResult<Self> {
        let mtm = MainThreadMarker::new().unwrap();
        let view = EngineContext::get()?.get_flutter_view(engine_handle)?;

        Ok(Self {
            id,
            weak_self: Late::new(),
            view: unsafe { Id::cast(view) },
            delegate,
            interaction: Late::new(),
            interaction_delegate: Late::new(),
            sessions: RefCell::new(HashMap::new()),
            fading_containers: RefCell::new(Vec::new()),
            mtm,
        })
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self.clone());
        let delegate = SNEMenuContext::new(weak_self, self.mtm);
        self.interaction_delegate.set(delegate.retain());
        let interaction = unsafe {
            UIContextMenuInteraction::initWithDelegate(
                self.mtm.alloc::<UIContextMenuInteraction>(),
                &Id::cast(delegate.clone()),
            )
        };
        unsafe {
            self.view
                .addInteraction(&Retained::cast(interaction.clone()))
        };
        self.interaction.set(interaction);

        unsafe {
            let recognizer = UIPanGestureRecognizer::initWithTarget_action(
                self.mtm.alloc::<UIPanGestureRecognizer>(),
                Some(&Id::cast(delegate.clone())),
                Some(sel!(onGesture:)),
            );
            recognizer.setDelaysTouchesBegan(false);
            recognizer.setDelaysTouchesEnded(false);
            recognizer.setCancelsTouchesInView(false);
            self.view.addGestureRecognizer(&recognizer);
        }
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
        let view_controller =
            unsafe { UIViewController::init(self.mtm.alloc::<UIViewController>()) };

        let menu = menu_configuration.menu.as_ref().unwrap().ui_menu.clone();
        let configuration = unsafe {
            let menu = Rc::new(menu); // Id is not Clone
            let action_provider =
                RcBlock::new(move |_suggested: NonNull<NSArray<UIMenuElement>>| {
                    Id::autorelease_return(menu.retain())
                });

            let preview_provider = match (
                menu_configuration.preview_image.as_ref(),
                menu_configuration.preview_size.as_ref(),
            ) {
                (Some(preview_image), None) => {
                    let preview_view = image_view_from_data(preview_image.clone(), self.mtm);
                    let size = CGSize {
                        width: preview_image.point_width(),
                        height: preview_image.point_height(),
                    };
                    let controller = view_controller.retain();
                    let preview_provider = RcBlock::new(move || {
                        controller.setView(Some(&preview_view));
                        controller.setPreferredContentSize(size);
                        Id::autorelease_return(controller.retain())
                    });
                    Some(preview_provider)
                }
                (None, Some(size)) => {
                    let controller = view_controller.retain();
                    let size: CGSize = size.clone().into();
                    let preview_provider = RcBlock::new(move || {
                        controller.setPreferredContentSize(size);
                        let view = controller.view();
                        if let Some(view) = view {
                            let is_image_view = view.is_kind_of::<UIImageView>();
                            if is_image_view {
                                return Id::autorelease_return(controller.retain());
                            }
                        }
                        let view = UIView::initWithFrame(self.mtm.alloc::<UIView>(), CGRect::ZERO);
                        let activity_indicator = {
                            let res = UIActivityIndicatorView::initWithActivityIndicatorStyle(
                                self.mtm.alloc::<UIActivityIndicatorView>(),
                                UIActivityIndicatorViewStyle::Medium,
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
                    Some(preview_provider)
                }
                _ => None,
            };

            UIContextMenuConfiguration::configurationWithIdentifier_previewProvider_actionProvider(
                None,
                preview_provider
                    .as_deref()
                    .map(|p| p as *const _ as *mut _)
                    .unwrap_or(std::ptr::null_mut()),
                action_provider.deref() as *const _ as *mut _,
                self.mtm,
            )
        };
        let view_container = unsafe {
            let bounds = self.view.bounds();
            let container = UIView::initWithFrame(self.mtm.alloc::<UIView>(), bounds);
            container.setUserInteractionEnabled(false);
            self.view.addSubview(&container);
            container
        };

        let session = MenuSession {
            _id: configuration.clone(),
            view_container,
            configuration: menu_configuration,
            view_controller,
            mtm: self.mtm,
        };
        self.sessions
            .borrow_mut()
            .insert(MenuSession::get_id(&configuration), session);
        configuration
    }

    fn on_pan_recognized(&self) {
        let containers = self.fading_containers.borrow();
        for container in containers.iter() {
            container.setHidden(true);
        }
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
                let lift_image = image_view_from_data(image.image_data.clone(), self.mtm);
                let frame: CGRect = image.rect.translated(-100000.0, -100000.0).into();
                lift_image.setFrame(frame);
                session.view_container.addSubview(&lift_image);
                let parameters = UIPreviewParameters::init(self.mtm.alloc::<UIPreviewParameters>());
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
                    self.mtm.alloc::<UIPreviewTarget>(),
                    &session.view_container,
                    center,
                );

                let preview = UITargetedPreview::initWithView_parameters_target(
                    self.mtm.alloc::<UITargetedPreview>(),
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
        self: &Rc<Self>,
        _interaction: &UIContextMenuInteraction,
        configuration: &UIContextMenuConfiguration,
        _animator: Option<&ProtocolObject<dyn UIContextMenuInteractionAnimating>>,
    ) {
        let session_id = MenuSession::get_id(configuration);
        let session = self.sessions.borrow_mut().remove(&session_id);
        if let Some(session) = session {
            unsafe {
                let container = session.view_container.clone();
                self.fading_containers.borrow_mut().push(container.clone());
                let container_clone = container.clone();
                let animation = RcBlock::new(move || {
                    container_clone.setAlpha(0.0);
                });

                let self_clone = self.clone();
                let completion = RcBlock::new(move |_| {
                    session.view_container.removeFromSuperview();
                    self_clone
                        .fading_containers
                        .borrow_mut()
                        .retain(|c| c != &container);
                });

                // Immediately fading out looks glitchy because it happens during menu -> lift
                // transition, but waiting until provided animator complete is called is too late.
                // So instead of using the provided animator, use custom animation block with delay.
                UIView::animateWithDuration_delay_options_animations_completion(
                    0.25,
                    0.25,
                    UIViewAnimationOptions::empty(),
                    &animation,
                    Some(&completion),
                    self.mtm,
                );
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
        unsafe {
            self.view
                .removeInteraction(&Retained::cast(self.interaction.clone()))
        };
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
    struct SNEMenuContext;

    unsafe impl ClassType for SNEMenuContext {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "SNEMenuContext";
    }

    impl DeclaredClass for SNEMenuContext {
        type Ivars = Inner;
    }

    unsafe impl NSObjectProtocol for SNEMenuContext {}

    #[allow(non_snake_case)]
    unsafe impl SNEMenuContext {
        #[method(onGesture:)]
        fn onGesture(&self, detector: &UIPanGestureRecognizer) {
            if detector.state() == UIGestureRecognizerState::Began {
                self.ivars().with_state(
                    |state| state.on_pan_recognized(),
                    || {},
                );
            }
        }
    }

    #[allow(non_snake_case)]
    unsafe impl UIContextMenuInteractionDelegate for SNEMenuContext {
        #[method_id(contextMenuInteraction:configurationForMenuAtLocation:)]
        fn contextMenuInteraction_configurationForMenuAtLocation(
            &self,
            interaction: &UIContextMenuInteraction,
            location: CGPoint,
        ) -> Option<Id<UIContextMenuConfiguration>> {
            self.ivars().with_state(
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
            self.ivars().with_state(
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
            self.ivars().with_state(
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
            self.ivars().with_state(
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
            self.ivars().with_state(
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
    fn new(context: Weak<PlatformMenuContext>, mtm: MainThreadMarker) -> Id<Self> {
        let this = mtm.alloc::<Self>();
        let this = this.set_ivars(Inner { context });
        unsafe { msg_send_id![super(this), init] }
    }
}
