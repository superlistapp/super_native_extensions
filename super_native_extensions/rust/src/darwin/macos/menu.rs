use std::rc::{Rc, Weak};

use block::ConcreteBlock;
use cocoa::{
    appkit::{NSEventModifierFlags, NSMenuItem},
    base::{id, nil},
    foundation::{NSInteger, NSPoint},
};
use irondash_engine_context::EngineContext;
use irondash_message_channel::IsolateId;
use irondash_run_loop::{spawn, util::FutureCompleter, RunLoop};
use objc::{
    class, msg_send,
    rc::StrongPtr,
    runtime::{Sel, BOOL, YES},
    sel, sel_impl,
};

use crate::{
    api_model::{
        Activator, ImageData, Menu, MenuElement, MenuImage, ShowContextMenuRequest,
        ShowContextMenuResponse,
    },
    error::NativeExtensionsResult,
    log::OkLog,
    menu_manager::{PlatformMenuContextDelegate, PlatformMenuContextId, PlatformMenuDelegate},
    platform_impl::platform::common::to_nsstring,
};

use super::util::{flip_position, ns_image_for_menu_item};

pub struct PlatformMenuContext {
    delegate: Weak<dyn PlatformMenuContextDelegate>,
    view: StrongPtr,
}

pub struct PlatformMenu {
    menu: StrongPtr,
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
        let mut res = NSEventModifierFlags::empty();
        if activator.alt {
            res |= NSEventModifierFlags::NSAlternateKeyMask;
        }
        if activator.meta {
            res |= NSEventModifierFlags::NSCommandKeyMask;
        }
        if activator.control {
            res |= NSEventModifierFlags::NSControlKeyMask;
        }
        if activator.shift {
            res |= NSEventModifierFlags::NSShiftKeyMask;
        }

        res
    }

    unsafe fn translate_menu(
        menu: &Menu,
        isolate: IsolateId,
        delegate: Weak<dyn PlatformMenuDelegate>,
    ) -> StrongPtr {
        let res: id = msg_send![class!(SNEMenu), alloc];
        let res: id =
            msg_send![res, initWithTitle: *to_nsstring(menu.title.as_deref().unwrap_or_default())];
        let res = StrongPtr::new(res);
        for child in &menu.children {
            let child = Self::translate_element(child, isolate, delegate.clone());
            let () = msg_send![*res, addItem:*child];
        }
        res
    }

    async unsafe fn load_deferred_menu_item(
        item: id,
        item_id: i64,
        isolate: IsolateId,
        weak_delegate: Weak<dyn PlatformMenuDelegate>,
    ) {
        if let Some(delegate) = weak_delegate.upgrade() {
            let item = StrongPtr::retain(item);
            let parent_menu = StrongPtr::retain(msg_send![*item, menu]);
            let elements = delegate.get_deferred_menu(isolate, item_id).await.ok_log();

            for element in elements.unwrap_or_default() {
                let element = Self::translate_element(&element, isolate, weak_delegate.clone());
                let index: NSInteger = msg_send![*parent_menu, indexOfItem: *item];
                let () = msg_send![*parent_menu, insertItem:*element atIndex:index];
            }
            let () = msg_send![*parent_menu, removeItem: *item];
        }
    }

    unsafe fn translate_element(
        element: &MenuElement,
        isolate: IsolateId,
        delegate: Weak<dyn PlatformMenuDelegate>,
    ) -> StrongPtr {
        match element {
            MenuElement::Action(menu_action) => {
                let delegate = delegate.clone();
                let item: id = msg_send![class!(SNEBlockMenuItem), alloc];
                let item: id = if menu_action.attributes.disabled {
                    msg_send![item, initWithTitle: *to_nsstring(menu_action.title.as_deref().unwrap_or_default())
                                    keyEquivalent: *to_nsstring("") block: nil]
                } else {
                    let action = menu_action.unique_id;
                    let action = move |_item: id| {
                        if let Some(delegate) = delegate.upgrade() {
                            delegate.on_action(isolate, action);
                        }
                    };
                    let action = ConcreteBlock::new(action);
                    let action = action.copy();
                    msg_send![item, initWithTitle: *to_nsstring(menu_action.title.as_deref().unwrap_or_default())
                                    keyEquivalent: *to_nsstring("") block: &*action]
                };

                if let Some(MenuImage::Image { data }) = &menu_action.image {
                    let image = ns_image_for_menu_item(data.clone());
                    let () = msg_send![item, setImage: *image];
                }

                if let Some(activator) = &menu_action.activator {
                    let str = Self::activator_label_to_string(activator);
                    if !str.is_empty() {
                        let () = msg_send![item, setKeyEquivalent: to_nsstring(&str)];
                        let () = msg_send![
                            item,
                            setKeyEquivalentModifierMask:
                                Self::accelerator_label_to_modifier_flags(activator)
                        ];
                    }
                }

                let state: NSInteger = match menu_action.state {
                    crate::api_model::MenuActionState::None => 0,
                    crate::api_model::MenuActionState::CheckOn => 1,
                    crate::api_model::MenuActionState::CheckOff => 0,
                    crate::api_model::MenuActionState::CheckMixed => -1,
                    crate::api_model::MenuActionState::RadioOn => 1,
                    crate::api_model::MenuActionState::RadioOff => 0,
                };
                let () = msg_send![item, setState: state];

                StrongPtr::new(item)
            }
            MenuElement::Menu(menu) => {
                let item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                    *to_nsstring(menu.title.as_deref().unwrap_or_default()),
                    Sel::from_ptr(std::ptr::null_mut()),
                    *to_nsstring(""),
                );
                let submenu = Self::translate_menu(menu, isolate, delegate.clone());
                NSMenuItem::setSubmenu_(item, *submenu);
                StrongPtr::new(item)
            }
            MenuElement::Deferred(item) => {
                let item_id = item.unique_id;
                let action = move |item: id| {
                    let delegate = delegate.clone();
                    spawn(async move {
                        Self::load_deferred_menu_item(item, item_id, isolate, delegate.clone())
                            .await;
                    });
                };
                let action = ConcreteBlock::new(action);
                let action = action.copy();

                let item: id = msg_send![class!(SNEDeferredMenuItem), alloc];
                let item: id = msg_send![item, initWithBlock: &*action];
                StrongPtr::new(item)
            }
            MenuElement::Separator(_) => {
                let res = NSMenuItem::separatorItem(nil);
                StrongPtr::retain(res)
            }
        }
    }

    pub fn new(
        isolate: IsolateId,
        delegate: Weak<dyn PlatformMenuDelegate>,
        menu: Menu,
    ) -> NativeExtensionsResult<Rc<Self>> {
        let menu = unsafe { Self::translate_menu(&menu, isolate, delegate) };
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
            view: unsafe { StrongPtr::retain(view) },
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

    fn synthetize_mouse_up_event(&self) {
        if let Some(delegate) = self.delegate.upgrade() {
            let drag_contexts = delegate.get_platform_drag_contexts();
            for context in drag_contexts {
                if *context.view == *self.view {
                    unsafe {
                        context.synthetize_mouse_up_event();
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
        unsafe {
            flip_position(*self.view, &mut position);
        }
        let (future, completer) = FutureCompleter::new();

        let menu = request.menu.unwrap().menu.clone();
        let view = self.view.clone();

        self.synthetize_mouse_up_event();

        let cb = move || {
            let item_selected: BOOL = unsafe {
                msg_send![*menu, popUpMenuPositioningItem:nil atLocation:position inView:*view]
            };
            completer.complete(Ok(ShowContextMenuResponse {
                item_selected: item_selected == YES,
            }));
        };

        // this method might possibly be invoked from dispatch_async.
        // Showing the popup menu from dispatch_async will block
        // the dispatch queue; Instead we schedule this on next run
        //  loop turn, which doesn't block the dispatch queue;
        RunLoop::current().schedule_next(cb).detach();
        future.await
    }
}
