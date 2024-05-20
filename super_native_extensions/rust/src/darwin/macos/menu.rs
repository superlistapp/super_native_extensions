use std::rc::{Rc, Weak};

use block2::{Block, RcBlock};
use irondash_engine_context::EngineContext;
use irondash_message_channel::IsolateId;
use irondash_run_loop::{spawn, util::FutureCompleter, RunLoop};
use objc2::{
    extern_class, extern_methods,
    mutability::MainThreadOnly,
    rc::{Allocated, Id},
    ClassType,
};
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType, NSMenu, NSMenuItem, NSView};
use objc2_foundation::{ns_string, MainThreadMarker, NSPoint, NSString, NSUInteger};

use crate::{
    api_model::{
        Activator, ImageData, Menu, MenuElement, MenuImage, ShowContextMenuRequest,
        ShowContextMenuResponse,
    },
    error::NativeExtensionsResult,
    log::OkLog,
    menu_manager::{PlatformMenuContextDelegate, PlatformMenuContextId, PlatformMenuDelegate},
};

use super::util::{flip_position, ns_image_for_menu_item};

pub struct PlatformMenuContext {
    delegate: Weak<dyn PlatformMenuContextDelegate>,
    view: Id<NSView>,
}

pub struct PlatformMenu {
    menu: Id<NSMenu>,
}

impl std::fmt::Debug for PlatformMenu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformMenu").finish()
    }
}

impl PlatformMenu {
    fn activator_label_to_string(activator: &Activator) -> String {
        let label = activator.trigger.to_lowercase();
        let value = match label.as_str() {
            // these must match labels from LogicalKeyboardKey
            "f1" => 0xF704,
            "f2" => 0xF705,
            "f3" => 0xF706,
            "f4" => 0xF707,
            "f5" => 0xF708,
            "f6" => 0xF709,
            "f7" => 0xF70A,
            "f8" => 0xF70B,
            "f9" => 0xF70C,
            "f10" => 0xF70D,
            "f11" => 0xF70E,
            "f12" => 0xF70F,
            "home" => 0xF729,
            "end" => 0xF72B,
            "insert" => 0xF727,
            "delete" => 0xF728,
            "backspace" => 0x0008,
            "page up" => 0xF72C,
            "page down" => 0xF72D,
            "space" => 0x0020,
            "tab" => 0x0009,
            "enter" => 0x000d,
            "arrow up" => 0xF700,
            "arrow down" => 0xF701,
            "arrow left" => 0xF702,
            "arrow right" => 0xF703,
            _ => label.chars().next().unwrap_or(0 as char) as u32,
        };
        let mut res = String::new();
        if value > 0 {
            res.push(std::char::from_u32(value).unwrap());
        }
        res
    }

    fn accelerator_label_to_modifier_flags(activator: &Activator) -> NSEventModifierFlags {
        let mut res: NSUInteger = 0;
        if activator.alt {
            res |= NSEventModifierFlags::NSEventModifierFlagOption.0;
        }
        if activator.meta {
            res |= NSEventModifierFlags::NSEventModifierFlagCommand.0;
        }
        if activator.control {
            res |= NSEventModifierFlags::NSEventModifierFlagControl.0;
        }
        if activator.shift {
            res |= NSEventModifierFlags::NSEventModifierFlagShift.0;
        }

        NSEventModifierFlags(res)
    }

    unsafe fn translate_menu(
        menu: &Menu,
        isolate: IsolateId,
        delegate: Weak<dyn PlatformMenuDelegate>,
        main_thread_marker: MainThreadMarker,
    ) -> Id<NSMenu> {
        let title = menu.title.as_deref().unwrap_or_default();
        let res = SNEMenu::initWithTitle(
            main_thread_marker.alloc::<SNEMenu>(),
            &NSString::from_str(title),
        );
        for child in &menu.children {
            let child =
                Self::translate_element(child, isolate, delegate.clone(), main_thread_marker);
            res.addItem(&child);
        }
        Id::into_super(res)
    }

    async unsafe fn load_deferred_menu_item(
        item: &NSMenuItem,
        item_id: i64,
        isolate: IsolateId,
        weak_delegate: Weak<dyn PlatformMenuDelegate>,
        main_thread_marker: MainThreadMarker,
    ) {
        if let Some(delegate) = weak_delegate.upgrade() {
            let parent_menu = item.menu();
            let Some(parent_menu) = parent_menu else {
                return;
            };
            let elements = delegate.get_deferred_menu(isolate, item_id).await.ok_log();

            for element in elements.unwrap_or_default() {
                let element = Self::translate_element(
                    &element,
                    isolate,
                    weak_delegate.clone(),
                    main_thread_marker,
                );
                let index = parent_menu.indexOfItem(item);
                parent_menu.insertItem_atIndex(&element, index);
            }
            parent_menu.removeItem(item);
        }
    }

    unsafe fn translate_element(
        element: &MenuElement,
        isolate: IsolateId,
        delegate: Weak<dyn PlatformMenuDelegate>,
        main_thread_marker: MainThreadMarker,
    ) -> Id<NSMenuItem> {
        match element {
            MenuElement::Action(menu_action) => {
                let title = menu_action.title.as_deref().unwrap_or_default();
                let delegate = delegate.clone();
                let item = if menu_action.attributes.disabled {
                    SNEBlockMenuItem::initWithTitle(
                        main_thread_marker.alloc::<SNEBlockMenuItem>(),
                        &NSString::from_str(title),
                        ns_string!(""),
                        None,
                    )
                } else {
                    let action = menu_action.unique_id;
                    let action = move |_item: *mut NSMenuItem| {
                        if let Some(delegate) = delegate.upgrade() {
                            delegate.on_action(isolate, action);
                        }
                    };
                    let action = RcBlock::new(action);
                    SNEBlockMenuItem::initWithTitle(
                        main_thread_marker.alloc::<SNEBlockMenuItem>(),
                        &NSString::from_str(title),
                        ns_string!(""),
                        Some(&action),
                    )
                };

                if let Some(MenuImage::Image { data }) = &menu_action.image {
                    let image = ns_image_for_menu_item(data.clone());
                    item.setImage(Some(&image));
                }

                if let Some(activator) = &menu_action.activator {
                    let str = Self::activator_label_to_string(activator);
                    if !str.is_empty() {
                        item.setKeyEquivalent(&NSString::from_str(&str));
                        item.setKeyEquivalentModifierMask(
                            Self::accelerator_label_to_modifier_flags(activator),
                        );
                    }
                }

                let state: isize = match menu_action.state {
                    crate::api_model::MenuActionState::None => 0,
                    crate::api_model::MenuActionState::CheckOn => 1,
                    crate::api_model::MenuActionState::CheckOff => 0,
                    crate::api_model::MenuActionState::CheckMixed => -1,
                    crate::api_model::MenuActionState::RadioOn => 1,
                    crate::api_model::MenuActionState::RadioOff => 0,
                };
                item.setState(state);
                Id::into_super(item)
            }
            MenuElement::Menu(menu) => {
                let title = menu.title.as_deref().unwrap_or_default();
                let item = NSMenuItem::initWithTitle_action_keyEquivalent(
                    main_thread_marker.alloc::<NSMenuItem>(),
                    &NSString::from_str(title),
                    None,
                    ns_string!(""),
                );

                if let Some(MenuImage::Image { data }) = &menu.image {
                    let image = ns_image_for_menu_item(data.clone());
                    item.setImage(Some(&image));
                }
                let submenu =
                    Self::translate_menu(menu, isolate, delegate.clone(), main_thread_marker);
                item.setSubmenu(Some(&submenu));
                item
            }
            MenuElement::Deferred(item) => {
                let item_id = item.unique_id;
                let action = move |item: *mut NSMenuItem| {
                    let item = unsafe { &*item };
                    let delegate = delegate.clone();
                    let item = item.retain();
                    spawn(async move {
                        Self::load_deferred_menu_item(
                            &item,
                            item_id,
                            isolate,
                            delegate.clone(),
                            main_thread_marker,
                        )
                        .await;
                    });
                };
                let action = RcBlock::new(action);

                let item = SNEDeferredMenuItem::initWithBlock(
                    main_thread_marker.alloc::<SNEDeferredMenuItem>(),
                    &action,
                );
                Id::into_super(item)
            }
            MenuElement::Separator(_) => NSMenuItem::separatorItem(main_thread_marker),
        }
    }

    pub fn new(
        isolate: IsolateId,
        delegate: Weak<dyn PlatformMenuDelegate>,
        menu: Menu,
    ) -> NativeExtensionsResult<Rc<Self>> {
        let main_thread_marker = MainThreadMarker::new().unwrap();
        let menu = unsafe { Self::translate_menu(&menu, isolate, delegate, main_thread_marker) };
        Ok(Rc::new(Self { menu }))
    }
}

impl PlatformMenuContext {
    pub fn new(
        _id: PlatformMenuContextId,
        engine_handle: i64,
        delegate: Weak<dyn PlatformMenuContextDelegate>,
    ) -> NativeExtensionsResult<Self> {
        let view = EngineContext::get()?.get_flutter_view(engine_handle)?;
        Ok(Self {
            delegate,
            view: unsafe { Id::cast(view) },
        })
    }

    pub fn update_preview_image(
        &self,
        _configuration_id: i64,
        _image_data: ImageData,
    ) -> NativeExtensionsResult<()> {
        Ok(())
    }

    pub fn assign_weak_self(&self, _weak_self: Weak<Self>) {}

    fn synthesize_mouse_up_event(&self) {
        if let Some(delegate) = self.delegate.upgrade() {
            let drag_contexts = delegate.get_platform_drag_contexts();
            for context in drag_contexts {
                if *context.view == *self.view {
                    unsafe {
                        context.synthesize_mouse_up_event();
                    }
                }
            }
        }
    }

    pub async fn show_context_menu(
        &self,
        request: ShowContextMenuRequest,
    ) -> NativeExtensionsResult<ShowContextMenuResponse> {
        let mut position: NSPoint = request.location.into();
        flip_position(&self.view, &mut position);

        let (future, completer) = FutureCompleter::new();

        let menu = request.menu.unwrap().menu.clone();
        let view = self.view.clone();

        // remember the modifier flags before showing the popup menu
        let flags_before = unsafe { NSEvent::modifierFlags_class() };

        self.synthesize_mouse_up_event();

        let cb = move || {
            let item_selected = unsafe {
                menu.popUpMenuPositioningItem_atLocation_inView(None, position, Some(&view))
            };
            // If the the popup menu was shown because of control + click and the
            // control is no longe pressed after menu is closed we need to let Flutter
            // know otherwise it will end up with control stuck.
            unsafe {
                let modifier_flags = NSEvent::modifierFlags_class();

                if (flags_before.0 & NSEventModifierFlags::NSEventModifierFlagControl.0
                    == NSEventModifierFlags::NSEventModifierFlagControl.0)
                    && (modifier_flags.0 & NSEventModifierFlags::NSEventModifierFlagControl.0 == 0)
                {
                    let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode
                    (NSEventType::FlagsChanged, NSPoint::ZERO, NSEventModifierFlags(0), 0.0, 0, None, ns_string!(""), ns_string!(""), false, 0).unwrap();
                    let window = view.window();
                    if let Some(window) = window {
                        window.sendEvent(&event);
                    }
                }
            }
            completer.complete(Ok(ShowContextMenuResponse { item_selected }));
        };

        // this method might possibly be invoked from dispatch_async.
        // Showing the popup menu from dispatch_async will block
        // the dispatch queue; Instead we schedule this on next run
        //  loop turn, which doesn't block the dispatch queue;
        RunLoop::current().schedule_next(cb).detach();
        future.await
    }
}

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub struct SNEMenu;

    unsafe impl ClassType for SNEMenu {
        type Super = NSMenu;
        type Mutability = MainThreadOnly;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub struct SNEBlockMenuItem;

    unsafe impl ClassType for SNEBlockMenuItem {
        type Super = NSMenuItem;
        type Mutability = MainThreadOnly;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub struct SNEDeferredMenuItem;

    unsafe impl ClassType for SNEDeferredMenuItem {
        type Super = NSMenuItem;
        type Mutability = MainThreadOnly;
    }
);

extern_methods!(
    unsafe impl SNEMenu {
        #[allow(non_snake_case)]
        #[method_id(@__retain_semantics Init initWithTitle:)]
        pub unsafe fn initWithTitle(this: Allocated<Self>, title: &NSString) -> Id<Self>;
    }

    unsafe impl SNEBlockMenuItem {
        #[allow(non_snake_case)]
        #[method_id(@__retain_semantics Init initWithTitle:keyEquivalent:block:)]
        pub unsafe fn initWithTitle(
            this: Allocated<Self>,
            title: &NSString,
            keyEquivalent: &NSString,
            block: Option<&Block<dyn Fn(*mut NSMenuItem)>>,
        ) -> Id<Self>;
    }

    unsafe impl SNEDeferredMenuItem {
        #[allow(non_snake_case)]
        #[method_id(@__retain_semantics Init initWithBlock:)]
        pub unsafe fn initWithBlock(
            this: Allocated<Self>,
            block: &Block<dyn Fn(*mut NSMenuItem)>,
        ) -> Id<Self>;
    }
);
