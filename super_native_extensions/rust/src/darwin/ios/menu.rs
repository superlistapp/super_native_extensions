use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::c_void,
    fmt::Formatter,
    mem::ManuallyDrop,
    rc::{Rc, Weak},
};

use block::{Block, ConcreteBlock, RcBlock};
use cocoa::{
    base::{id, nil, BOOL, NO},
    foundation::{NSArray, NSInteger, NSUInteger},
};

use core_graphics::geometry::{CGPoint, CGRect, CGSize};
use irondash_engine_context::EngineContext;
use irondash_message_channel::{IsolateId, Late};
use irondash_run_loop::{platform::PollSession, spawn, RunLoop};

use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    rc::{autoreleasepool, StrongPtr},
    runtime::{Class, Object, Protocol, Sel},
    sel, sel_impl,
};
use once_cell::sync::Lazy;

use crate::{
    api_model::{
        ImageData, Menu, MenuActionState, MenuConfiguration, MenuElement, MenuImage, Rect,
    },
    error::NativeExtensionsResult,
    menu_manager::{PlatformMenuContextDelegate, PlatformMenuContextId, PlatformMenuDelegate},
    platform_impl::platform::{
        common::{superclass, to_nsstring},
        os::util::{image_view_from_data, IgnoreInteractionEvents},
    },
    value_promise::PromiseResult,
};

use super::{_CGPoint, alpha_to_path::bezier_path_for_alpha, util::image_from_image_data};

pub struct PlatformMenuContext {
    id: PlatformMenuContextId,
    weak_self: Late<Weak<Self>>,
    view: StrongPtr,
    delegate: Weak<dyn PlatformMenuContextDelegate>,
    interaction: Late<StrongPtr>,
    interaction_delegate: Late<StrongPtr>,
    sessions: RefCell<HashMap<usize, MenuSession>>,
}

pub struct PlatformMenu {
    ui_menu: StrongPtr,
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
        let res = Self {
            ui_menu: autoreleasepool(|| unsafe {
                Self::convert_menu(MenuElement::Menu(menu), isolate, &delegate)
            })?,
        };
        Ok(Rc::new(res))
    }

    unsafe fn convert_string(str: &Option<String>) -> StrongPtr {
        match str {
            Some(str) => to_nsstring(str),
            None => StrongPtr::new(nil),
        }
    }

    unsafe fn convert_image(image: &Option<MenuImage>) -> StrongPtr {
        match image {
            // Some(image) => image_from_image_data(image.clone()),
            Some(MenuImage::Image { data }) => image_from_image_data(data.clone()),
            Some(MenuImage::System { name }) => {
                let name = to_nsstring(name);
                let res: id = msg_send![class!(UIImage), systemImageNamed: *name];
                StrongPtr::retain(res)
            }
            None => StrongPtr::new(nil),
        }
    }

    unsafe fn convert_elements(
        elements: Vec<MenuElement>,
        isolate_id: IsolateId,
        delegate: &Weak<dyn PlatformMenuDelegate>,
    ) -> NativeExtensionsResult<Vec<id>> {
        let mut res = Vec::new();

        struct InlineSection {
            title: StrongPtr,
            elements: Vec<id>,
        }

        let mut inline_section = None::<InlineSection>;

        unsafe fn finish_inline_section(inline_section: Option<InlineSection>) -> Vec<id> {
            if let Some(inline_section) = inline_section {
                let elements = NSArray::arrayWithObjects(nil, &inline_section.elements);
                let res: id = msg_send![class!(UIMenu),
                    menuWithTitle:inline_section.title.autorelease()
                    image:nil
                    identifier:nil
                    options:1 as NSUInteger // UIMenuOptionsDisplayInline
                    children:elements];
                vec![res]
            } else {
                Vec::new()
            }
        }

        for element in elements {
            match element {
                MenuElement::Separator(separator) => {
                    res.append(&mut finish_inline_section(inline_section));
                    inline_section = Some(InlineSection {
                        title: Self::convert_string(&separator.title),
                        elements: Vec::new(),
                    });
                }
                element => {
                    let converted = Self::convert_menu(element, isolate_id, delegate)?;
                    if let Some(inline_section) = inline_section.as_mut() {
                        inline_section.elements.push(converted.autorelease());
                    } else {
                        res.push(converted.autorelease());
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
    ) -> NativeExtensionsResult<StrongPtr> {
        match menu {
            MenuElement::Action(action) => {
                let unique_id = action.unique_id;
                let delegate = delegate.clone();
                let handler = ConcreteBlock::new(move |_: id| {
                    autoreleasepool(|| {
                        if let Some(delegate) = delegate.upgrade() {
                            delegate.on_action(isolate_id, unique_id);
                        }
                    });
                });
                let handler = handler.copy();
                let res: id = msg_send![class!(UIAction),
                    actionWithTitle:*Self::convert_string(&action.title)
                    image:*Self::convert_image(&action.image)
                    identifier:*Self::convert_string(&action.identifier)
                    handler:&*handler];

                let mut options = 0 as NSUInteger;
                if action.attributes.disabled {
                    options |= 1 << 0; // UIMenuElementAttributesDisabled
                }
                if action.attributes.destructive {
                    options |= 1 << 1; // UIMenuElementAttributesDestructive
                }
                unsafe {
                    let _: () = msg_send![res, setAttributes: options];
                }

                let state: NSUInteger = match action.state {
                    MenuActionState::None => 0,
                    MenuActionState::CheckOff => 0,
                    MenuActionState::RadioOff => 0,
                    MenuActionState::CheckOn => 1,
                    MenuActionState::RadioOn => 1,
                    MenuActionState::CheckMixed => 2,
                };

                unsafe {
                    let _: () = msg_send![res, setState: state];
                }

                Ok(StrongPtr::retain(res))
            }
            MenuElement::Menu(menu) => {
                let children = Self::convert_elements(menu.children, isolate_id, delegate)?;
                let children = NSArray::arrayWithObjects(nil, &children);
                let res: id = msg_send![class!(UIMenu),
                    menuWithTitle:*Self::convert_string(&menu.title)
                    image:*Self::convert_image(&menu.image)
                    identifier:*Self::convert_string(&menu.identifier)
                    options:0 as NSUInteger
                    children:children];
                Ok(StrongPtr::retain(res))
            }
            MenuElement::Deferred(deferred) => {
                let delegate = delegate.clone();
                let provider = ConcreteBlock::new(move |completion_block: id| -> id {
                    let delegate = delegate.clone();
                    let completion_block =
                        unsafe { &mut *(completion_block as *mut Block<(id,), ()>) };
                    let completion_block = unsafe { RcBlock::copy(completion_block) };
                    spawn(async move {
                        if let Some(delegate) = delegate.upgrade() {
                            let menu = delegate
                                .get_deferred_menu(isolate_id, deferred.unique_id)
                                .await;
                            autoreleasepool(|| unsafe {
                                // let completion_block = completion_block.clone();
                                match menu {
                                    Ok(elements) => {
                                        let elements = Self::convert_elements(
                                            elements,
                                            isolate_id,
                                            &Rc::downgrade(&delegate),
                                        );
                                        match elements {
                                            Ok(elements) => {
                                                let elements =
                                                    NSArray::arrayWithObjects(nil, &elements);
                                                completion_block.call((elements,));
                                            }
                                            Err(_) => completion_block.call((nil,)),
                                        }
                                    }
                                    Err(_) => completion_block.call((nil,)),
                                }
                            });
                        }
                    });
                    nil
                });
                let provider = provider.copy();
                let res: id = msg_send![class!(UIDeferredMenuElement),
                    elementWithProvider:&*provider];
                Ok(StrongPtr::retain(res))
            }
            MenuElement::Separator(_separator) => {
                panic!("Separator should be converted to inline section")
            }
        }
    }
}

struct MenuSession {
    _id: StrongPtr,
    view_container: StrongPtr,
    view_controller: StrongPtr,
    configuration: MenuConfiguration,
}

impl MenuSession {
    fn update_preview_image(&self, image: ImageData) {
        let preview_view = image_view_from_data(image);
        unsafe {
            let view: id = msg_send![*self.view_controller, view];
            let sub_views: id = msg_send![view, subviews];
            let prev_subview = StrongPtr::retain(NSArray::objectAtIndex(sub_views, 0));
            let () = msg_send![view, addSubview:*preview_view];
            let () = msg_send![*preview_view, setAlpha: 0.0];

            let prev_copy = prev_subview.clone();
            let animation = ConcreteBlock::new(move || {
                let () = msg_send![*preview_view, setAlpha: 1.0];
                let () = msg_send![*prev_copy, setAlpha: 0.0];
            });
            let animation = animation.copy();
            let completion = ConcreteBlock::new(move |_: BOOL| {
                let () = msg_send![*prev_subview, removeFromSuperview];
            });
            let completion = completion.copy();
            let () = msg_send![class!(UIView), animateWithDuration:0.25 animations:&*animation completion:&*completion];
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
            view: unsafe { StrongPtr::retain(view) },
            delegate,
            interaction: Late::new(),
            interaction_delegate: Late::new(),
            sessions: RefCell::new(HashMap::new()),
        })
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self.clone());
        autoreleasepool(|| unsafe {
            let delegate: id = msg_send![*DELEGATE_CLASS, new];
            (*delegate).set_ivar("context", Weak::into_raw(weak_self) as *mut c_void);
            self.interaction_delegate.set(StrongPtr::new(delegate));
            let interaction: id = msg_send![class!(UIContextMenuInteraction), alloc];
            let interaction: id = msg_send![interaction, initWithDelegate: delegate];
            self.interaction.set(StrongPtr::new(interaction));
            let () = msg_send![*self.view, addInteraction: interaction];
        });
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

    fn _configuration_for_menu_at_location(
        &self,
        _interaction: id,
        menu_configuration: MenuConfiguration,
    ) -> id {
        let view_controller = unsafe {
            let res: id = msg_send![class!(UIViewController), alloc];
            StrongPtr::new(msg_send![res, init])
        };

        let menu = menu_configuration.menu.as_ref().unwrap().ui_menu.clone();
        let configuration = unsafe {
            let action_provider = ConcreteBlock::new(move |_: id| *menu);
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
                    let controller = view_controller.clone();
                    let preview_provider = ConcreteBlock::new(move || {
                        let () = msg_send![*controller, setView: *preview_view];
                        let () = msg_send![*controller, setPreferredContentSize: size];
                        controller.clone().autorelease()
                    });
                    let preview_provider = preview_provider.copy();
                    Some(preview_provider)
                }
                (None, Some(size)) => {
                    let controller = view_controller.clone();
                    let size: CGSize = size.clone().into();
                    let preview_provider = ConcreteBlock::new(move || {
                        let () = msg_send![*controller, setPreferredContentSize: size];
                        let view: id = msg_send![class!(UIView), alloc];
                        let view = StrongPtr::new(msg_send![
                            view,
                            initWithFrame: CGRect::from(Rect::default())
                        ]);
                        let activity_indicator = {
                            let res: id = msg_send![class!(UIActivityIndicatorView), alloc];
                            let res =
                                StrongPtr::new(msg_send![res, initWithActivityIndicatorStyle:
                                    100 as NSInteger]);
                            let () = msg_send![*res, startAnimating];
                            let () = msg_send![*res, setCenter: CGPoint {
                                x: size.width / 2.0,
                                y: size.height / 2.0,
                            }];
                            let white: id = msg_send![class!(UIColor), whiteColor];
                            let () = msg_send![*res, setColor: white];
                            res
                        };
                        let () = msg_send![*view, addSubview:*activity_indicator];
                        let () = msg_send![*controller, setView: *view];
                        controller.clone().autorelease()
                    });
                    let preview_provider = preview_provider.copy();
                    Some(preview_provider)
                }
                _ => None,
            };

            let conf: id = match preview_provider {
                Some(preview_provider) => {
                    msg_send![class!(UIContextMenuConfiguration),
                        configurationWithIdentifier:nil
                        previewProvider:preview_provider
                        actionProvider:&*action_provider]
                }
                None => {
                    msg_send![class!(UIContextMenuConfiguration),
                        configurationWithIdentifier:nil
                        previewProvider:nil
                        actionProvider:&*action_provider]
                }
            };
            StrongPtr::retain(conf)
        };
        let view_container = unsafe {
            let bounds: CGRect = msg_send![*self.view, bounds];
            let container: id = msg_send![class!(UIView), alloc];
            let container = StrongPtr::new(msg_send![container, initWithFrame: bounds]);
            let () = msg_send![*container, setUserInteractionEnabled: NO];
            let () = msg_send![*self.view, addSubview: *container];
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
            .insert(*configuration as usize, session);
        *configuration
    }

    fn configuration_for_menu_at_location(&self, interaction: id, location: CGPoint) -> id {
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
                            return self._configuration_for_menu_at_location(interaction, value);
                        }
                        PromiseResult::Cancelled => {
                            return nil;
                        }
                    }
                }
                RunLoop::current()
                    .platform_run_loop
                    .poll_once(&mut poll_session);
            }
        } else {
            nil
        }
    }

    fn preview_for_highlighting_menu_with_configuration(
        &self,
        _interaction: id,
        configuration: id,
    ) -> id {
        let sessions = self.sessions.borrow();
        let session = sessions.get(&(configuration as usize));
        match session {
            Some(session) => unsafe {
                let image = &session.configuration.lift_image;

                let lift_image = image_view_from_data(image.image_data.clone());

                let frame: CGRect = image.rect.translated(-100000.0, -100000.0).into();

                let () = msg_send![*lift_image, setFrame: frame];

                let () = msg_send![*session.view_container, addSubview:*lift_image];

                let parameters: id = msg_send![class!(UIPreviewParameters), new];
                let () = msg_send![parameters, autorelease];

                let shadow_path = bezier_path_for_alpha(&image.image_data);
                let () = msg_send![parameters, setShadowPath: *shadow_path];

                let clear_color: id = msg_send![class!(UIColor), clearColor];
                let () = msg_send![parameters, setBackgroundColor: clear_color];

                let target: id = msg_send![class!(UIPreviewTarget), alloc];
                let center: CGPoint = image.rect.center().into();
                let () = msg_send![target, initWithContainer:*session.view_container center:center];
                let () = msg_send![target, autorelease];

                let preview: id = msg_send![class!(UITargetedPreview), alloc];
                let () = msg_send![preview, initWithView:*lift_image parameters:parameters target:target];
                let () = msg_send![preview, autorelease];
                preview
            },
            _ => nil,
        }
    }

    fn interaction_will_display_menu_for_configuration(
        &self,
        _interaction: id,
        configuration: id,
        _animator: id,
    ) {
        let sessions = self.sessions.borrow();
        let session = sessions.get(&(configuration as usize));
        let delegate = self.delegate.upgrade();
        if let (Some(session), Some(delegate)) = (session, delegate) {
            delegate.on_show_menu(self.id, session.configuration.configuration_id);
        }
    }

    fn interaction_will_perform_preview_action_for_menu_with_configuration(
        &self,
        _interaction: id,
        configuration: id,
        _animator: id,
    ) {
        let sessions = self.sessions.borrow();
        let session = sessions.get(&(configuration as usize));
        let delegate = self.delegate.upgrade();
        if let (Some(session), Some(delegate)) = (session, delegate) {
            delegate.on_preview_action(self.id, session.configuration.configuration_id);
        }
    }

    fn interaction_will_end_for_configuration(
        &self,
        _interaction: id,
        configuration: id,
        animator: id,
    ) {
        let session = self.sessions.borrow_mut().remove(&(configuration as usize));
        if let Some(session) = session {
            unsafe {
                let container = session.view_container.clone();
                let animation = ConcreteBlock::new(move || {
                    let () = msg_send![*container, setAlpha:0.0];
                });
                let animation = animation.copy();
                let () = msg_send![animator, addAnimations: &*animation];

                let completion = ConcreteBlock::new(move |_finished: BOOL| {
                    let () = msg_send![*session.view_container, removeFromSuperview];
                });
                let completion = completion.copy();
                let () = msg_send![animator, addCompletion: &*completion];
            }
            if let Some(delegate) = self.delegate.upgrade() {
                delegate.on_hide_menu(self.id, session.configuration.configuration_id);
            }
        }
    }
}

extern "C" fn dealloc(this: &Object, _sel: Sel) {
    unsafe {
        let context_ptr = {
            let context_ptr: *mut c_void = *this.get_ivar("context");
            context_ptr as *const PlatformMenuContext
        };
        Weak::from_raw(context_ptr);

        let superclass = superclass(this);
        let () = msg_send![super(this, superclass), dealloc];
    }
}

fn with_state<F, FR, R>(this: id, callback: F, default: FR) -> R
where
    F: FnOnce(Rc<PlatformMenuContext>) -> R,
    FR: FnOnce() -> R,
{
    unsafe {
        let context_ptr = {
            let context_ptr: *mut c_void = *(*this).get_ivar("context");
            context_ptr as *const PlatformMenuContext
        };
        let this = ManuallyDrop::new(Weak::from_raw(context_ptr));
        let this = this.upgrade();
        match this {
            Some(this) => callback(this),
            None => default(),
        }
    }
}

extern "C" fn configuration_for_menu_at_location(
    this: &mut Object,
    _sel: Sel,
    interaction: id,
    location: _CGPoint,
) -> id {
    with_state(
        this,
        |state| state.configuration_for_menu_at_location(interaction, location.into()),
        || nil,
    )
}

extern "C" fn preview_for_highlighting_menu_with_configuration(
    this: &mut Object,
    _sel: Sel,
    interaction: id,
    configuration: id,
) -> id {
    with_state(
        this,
        |state| state.preview_for_highlighting_menu_with_configuration(interaction, configuration),
        || nil,
    )
}

extern "C" fn interaction_will_perform_preview_action_for_menu_with_configuration(
    this: &mut Object,
    _sel: Sel,
    interaction: id,
    configuration: id,
    animator: id,
) {
    with_state(
        this,
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

extern "C" fn interaction_will_display_menu_for_configuration(
    this: &mut Object,
    _sel: Sel,
    interaction: id,
    configuration: id,
    animator: id,
) {
    with_state(
        this,
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

extern "C" fn interaction_will_end_for_configuration(
    this: &mut Object,
    _sel: Sel,
    interaction: id,
    configuration: id,
    animator: id,
) {
    with_state(
        this,
        |state| state.interaction_will_end_for_configuration(interaction, configuration, animator),
        || {},
    );
}

static DELEGATE_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("SNEContextMenuInteractionDelegate", superclass).unwrap();
    decl.add_protocol(Protocol::get("UIContextMenuInteractionDelegate").unwrap());
    decl.add_ivar::<*mut c_void>("context");
    decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
    decl.add_method(
        sel!(contextMenuInteraction:willDisplayMenuForConfiguration:animator:),
        interaction_will_display_menu_for_configuration
            as extern "C" fn(&mut Object, Sel, id, id, id),
    );
    decl.add_method(
        sel!(contextMenuInteraction:configurationForMenuAtLocation:),
        configuration_for_menu_at_location as extern "C" fn(&mut Object, Sel, id, _CGPoint) -> id,
    );
    decl.add_method(
        sel!(contextMenuInteraction:willPerformPreviewActionForMenuWithConfiguration:animator:),
        interaction_will_perform_preview_action_for_menu_with_configuration
            as extern "C" fn(&mut Object, Sel, id, id, id),
    );
    decl.add_method(
        sel!(contextMenuInteraction:previewForHighlightingMenuWithConfiguration:),
        preview_for_highlighting_menu_with_configuration
            as extern "C" fn(&mut Object, Sel, id, id) -> id,
    );
    decl.add_method(
        sel!(contextMenuInteraction:willEndForConfiguration:animator:),
        interaction_will_end_for_configuration as extern "C" fn(&mut Object, Sel, id, id, id),
    );
    decl.register()
});
