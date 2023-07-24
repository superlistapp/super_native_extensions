use std::{
    cell::{Cell, RefCell},
    fmt::Write,
    rc::{Rc, Weak},
};

use gdk::{
    glib::{translate::from_glib_none, WeakRef},
    prelude::Cast,
    Event, Gravity, ModifierType, Rectangle,
};
use gtk::{
    traits::{
        AccelLabelExt, BinExt, BoxExt, CheckMenuItemExt, ContainerExt, GtkMenuExt, GtkMenuItemExt,
        LabelExt, MenuShellExt, SpinnerExt, WidgetExt,
    },
    Widget,
};
use gtk_sys::GtkWidget;
use irondash_engine_context::EngineContext;
use irondash_message_channel::IsolateId;
use irondash_run_loop::{spawn, util::FutureCompleter};

use crate::{
    api_model::{
        Activator, ImageData, Menu, MenuActionState, MenuElement, MenuImage,
        ShowContextMenuRequest, ShowContextMenuResponse,
    },
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    menu_manager::{PlatformMenuContextDelegate, PlatformMenuContextId, PlatformMenuDelegate},
};

use super::common::{surface_from_image_data, synthesize_button_up};

pub struct PlatformMenuContext {
    _delegate: Weak<dyn PlatformMenuContextDelegate>,
    view: WeakRef<Widget>,
}

pub struct PlatformMenu {
    menu: gtk::Menu,
    item_selected: Rc<Cell<bool>>,
}

impl std::fmt::Debug for PlatformMenu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformMenu").finish()
    }
}

struct MenuContext {
    item_selected: Rc<Cell<bool>>,
    on_menu_open_callbacks: Vec<Box<dyn FnOnce(&gtk::Menu)>>,
}

impl MenuContext {
    fn new(item_selected: Rc<Cell<bool>>) -> Self {
        Self {
            item_selected,
            on_menu_open_callbacks: Vec::new(),
        }
    }

    fn on_menu_open<F: FnOnce(&gtk::Menu) + 'static>(&mut self, f: F) {
        self.on_menu_open_callbacks.push(Box::new(f));
    }
}

impl PlatformMenu {
    fn activator_label_code(activator: &Activator) -> u32 {
        let label = activator.trigger.to_lowercase();
        let value = match label.as_str() {
            // these must match labels from LogicalKeyboardKey
            "f1" => gdk_sys::GDK_KEY_F1,
            "f2" => gdk_sys::GDK_KEY_F2,
            "f3" => gdk_sys::GDK_KEY_F3,
            "f4" => gdk_sys::GDK_KEY_F4,
            "f5" => gdk_sys::GDK_KEY_F5,
            "f6" => gdk_sys::GDK_KEY_F6,
            "f7" => gdk_sys::GDK_KEY_F7,
            "f8" => gdk_sys::GDK_KEY_F8,
            "f9" => gdk_sys::GDK_KEY_F9,
            "f10" => gdk_sys::GDK_KEY_F10,
            "f11" => gdk_sys::GDK_KEY_F11,
            "f12" => gdk_sys::GDK_KEY_F12,
            "home" => gdk_sys::GDK_KEY_Home,
            "end" => gdk_sys::GDK_KEY_End,
            "insert" => gdk_sys::GDK_KEY_Insert,
            "delete" => gdk_sys::GDK_KEY_Delete,
            "backspace" => gdk_sys::GDK_KEY_BackSpace,
            "page up" => gdk_sys::GDK_KEY_Page_Up,
            "page down" => gdk_sys::GDK_KEY_Page_Down,
            "space" => gdk_sys::GDK_KEY_space,
            "tab" => gdk_sys::GDK_KEY_Tab,
            "enter" => gdk_sys::GDK_KEY_Return,
            "arrow up" => gdk_sys::GDK_KEY_Up,
            "arrow down" => gdk_sys::GDK_KEY_Down,
            "arrow left" => gdk_sys::GDK_KEY_Left,
            "arrow right" => gdk_sys::GDK_KEY_Right,
            _ => label.chars().next().unwrap_or(0 as char) as i32,
        };
        value as u32
    }

    fn activator_modifier_type(activator: &Activator) -> ModifierType {
        let mut res = ModifierType::empty();
        if activator.alt {
            res |= ModifierType::MOD1_MASK;
        }
        if activator.meta {
            res |= ModifierType::META_MASK;
        }
        if activator.control {
            res |= ModifierType::CONTROL_MASK;
        }
        if activator.shift {
            res |= ModifierType::SHIFT_MASK;
        }
        res
    }

    // Convert & mnemonics to _
    #[allow(clippy::branches_sharing_code)]
    fn convert_mnemonics(title: &str) -> String {
        let mut res = String::new();
        let mut mnemonic = false;
        for c in title.chars() {
            if c == '&' {
                if !mnemonic {
                    mnemonic = true;
                    continue;
                } else {
                    res.write_char('&').unwrap();
                    mnemonic = false;
                    continue;
                }
            }
            if mnemonic {
                res.write_char('_').unwrap();
                mnemonic = false;
            }
            res.write_char(c).unwrap();

            if c == '_' {
                res.write_char('_').unwrap();
            }
        }
        res
    }

    fn translate_menu(
        menu: &Menu,
        item_selected: Rc<Cell<bool>>,
        isolate: IsolateId,
        delegate: &Weak<dyn PlatformMenuDelegate>,
    ) -> gtk::Menu {
        let res = gtk::Menu::new();
        let mut context = MenuContext::new(item_selected);
        for element in &menu.children {
            let menu_item = Self::translate_menu_element(element, &mut context, isolate, delegate);
            res.add(&menu_item);
        }

        let callbacks = Rc::new(RefCell::new(Some(context.on_menu_open_callbacks)));
        res.connect_show(move |menu| {
            if let Some(callbacks) = callbacks.take() {
                for callback in callbacks {
                    callback(menu);
                }
            }
        });
        res.show_all();
        res
    }

    fn translate_menu_element(
        element: &MenuElement,
        context: &mut MenuContext,
        isolate: IsolateId,
        delegate: &Weak<dyn PlatformMenuDelegate>,
    ) -> gtk::MenuItem {
        match element {
            MenuElement::Action(action) => {
                let item = match &action.state {
                    MenuActionState::None => gtk::MenuItem::new(),
                    state => {
                        let res = gtk::CheckMenuItem::new();
                        res.set_active(
                            state == &MenuActionState::CheckOn
                                || state == &MenuActionState::RadioOn,
                        );
                        res.set_inconsistent(state == &MenuActionState::CheckMixed);
                        res.set_draw_as_radio(
                            state == &MenuActionState::RadioOn
                                || state == &MenuActionState::RadioOff,
                        );
                        res.upcast()
                    }
                };

                let label = if let Some(MenuImage::Image { data }) = &action.image {
                    let surface = surface_from_image_data(data.clone(), 1.0);
                    let image = gtk::Image::from_surface(Some(&surface));
                    let item_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
                    item_box.add(&image);
                    let label = gtk::AccelLabel::new(&Self::convert_mnemonics(
                        action.title.as_deref().unwrap_or_default(),
                    ));
                    label.set_use_underline(true);
                    label.set_xalign(0.0);
                    item_box.pack_end(&label, true, true, 0);
                    item.add(&item_box);
                    label
                } else {
                    item.set_label(&Self::convert_mnemonics(
                        action.title.as_deref().unwrap_or_default(),
                    ));
                    item.child()
                        .and_then(|c| c.downcast::<gtk::AccelLabel>().ok())
                        .unwrap()
                };

                if let Some(activator) = &action.activator {
                    label.set_accel(
                        Self::activator_label_code(activator),
                        Self::activator_modifier_type(activator),
                    );
                }

                item.set_sensitive(!action.attributes.disabled);

                let item_selected = context.item_selected.clone();
                let delegate = delegate.clone();
                let unique_id = action.unique_id;
                item.connect_activate(move |_| {
                    if let Some(delegate) = delegate.upgrade() {
                        item_selected.set(true);
                        delegate.on_action(isolate, unique_id);
                    }
                });
                item
            }
            MenuElement::Menu(menu) => {
                let item = gtk::MenuItem::new();
                item.set_label(&Self::convert_mnemonics(
                    menu.title.as_deref().unwrap_or_default(),
                ));
                let submenu =
                    Self::translate_menu(menu, context.item_selected.clone(), isolate, delegate);
                item.set_submenu(Some(&submenu));
                item
            }
            MenuElement::Deferred(deferred) => {
                let item = gtk::MenuItem::new();
                let item_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                let spinner = gtk::Spinner::new();
                spinner.start();
                item_box.add(&spinner);
                item.add(&item_box);
                item.set_sensitive(false);

                let item_clone = item.clone();
                let delegate = delegate.clone();
                let unique_id = deferred.unique_id;
                let item_selected = context.item_selected.clone();
                context.on_menu_open(move |menu| {
                    if let Some(delegate) = delegate.upgrade() {
                        let menu = menu.clone();
                        spawn(async move {
                            Self::load_deferred_menu_item(
                                delegate,
                                isolate,
                                unique_id,
                                menu,
                                item_clone,
                                item_selected,
                            )
                            .await;
                        });
                    }
                });

                item
            }
            MenuElement::Separator(_) => gtk::SeparatorMenuItem::new().upcast(),
        }
    }

    async fn load_deferred_menu_item(
        delegate: Rc<dyn PlatformMenuDelegate>,
        isolate: IsolateId,
        item_id: i64,
        menu: gtk::Menu,
        deferred_item: gtk::MenuItem,
        item_selected: Rc<Cell<bool>>,
    ) {
        if let Some(result) = delegate.get_deferred_menu(isolate, item_id).await.ok_log() {
            let mut current_index = 0;
            let mut actual_index = None::<i32>;
            menu.forall(|item| {
                if item == &deferred_item {
                    actual_index = Some(current_index);
                }
                current_index += 1;
            });

            if let Some(mut index) = actual_index {
                let mut context = MenuContext::new(item_selected);
                for element in result {
                    let translated = Self::translate_menu_element(
                        &element,
                        &mut context,
                        isolate,
                        &Rc::downgrade(&delegate),
                    );
                    translated.show();
                    menu.insert(&translated, index);
                    index += 1;
                }
            }
            menu.remove(&deferred_item);

            let top_level = menu.toplevel();
            let win = top_level.as_ref().and_then(|w| w.window());
            if let (Some(win), Some(top_level)) = (win, top_level) {
                if win.is_visible() {
                    let natural_size = top_level.preferred_size().1;
                    win.resize(natural_size.width, natural_size.height);
                }
            }
        }
    }

    pub fn new(
        isolate: IsolateId,
        delegate: Weak<dyn PlatformMenuDelegate>,
        menu: Menu,
    ) -> NativeExtensionsResult<Rc<Self>> {
        let item_selected = Rc::new(Cell::new(false));
        let menu = Self::translate_menu(&menu, item_selected.clone(), isolate, &delegate);
        Ok(Rc::new(Self {
            menu,
            item_selected,
        }))
    }
}

impl PlatformMenuContext {
    pub fn new(
        _id: PlatformMenuContextId,
        engine_handle: i64,
        delegate: Weak<dyn PlatformMenuContextDelegate>,
    ) -> NativeExtensionsResult<Self> {
        unsafe { gtk::set_initialized() };

        let view = EngineContext::get()?.get_flutter_view(engine_handle)?;
        let view: Widget = unsafe { from_glib_none(view as *mut GtkWidget) };
        let weak = WeakRef::new();
        weak.set(Some(&view));

        Ok(Self {
            _delegate: delegate,
            view: weak,
        })
    }

    pub fn assign_weak_self(&self, _weak_self: Weak<Self>) {}

    pub fn update_preview_image(
        &self,
        _configuration_id: i64,
        _image_data: ImageData,
    ) -> NativeExtensionsResult<()> {
        Err(NativeExtensionsError::UnsupportedOperation)
    }

    fn last_event(&self) -> Option<Event> {
        let delegate = self._delegate.upgrade()?;
        for d in delegate.get_platform_drag_contexts() {
            if d.view == self.view {
                return d.last_button_press_event.borrow().as_ref().cloned();
            }
        }
        None
    }

    pub async fn show_context_menu(
        &self,
        request: ShowContextMenuRequest,
    ) -> NativeExtensionsResult<ShowContextMenuResponse> {
        let platform_menu = request
            .menu
            .ok_or(NativeExtensionsError::PlatformMenuNotFound)?;
        let event = self
            .last_event()
            .ok_or(NativeExtensionsError::MouseEventNotFound)?;

        let mut release = synthesize_button_up(&event);
        gtk::main_do_event(&mut release);

        let view = self.view.upgrade().unwrap();

        let menu = platform_menu.menu.clone();
        menu.popup_at_rect(
            &view.window().unwrap(),
            &Rectangle::new(
                request.location.x as i32 + view.allocated_size().0.x(),
                request.location.y as i32 + view.allocated_size().0.y(),
                0,
                0,
            ),
            Gravity::SouthWest,
            Gravity::NorthWest,
            Some(&event),
        );
        let (future, completer) = FutureCompleter::new();
        let completer = Rc::new(RefCell::new(Some(completer)));

        let item_selected = platform_menu.item_selected.clone();
        menu.connect_selection_done(move |_menu| {
            completer.take().unwrap().complete(ShowContextMenuResponse {
                item_selected: item_selected.get(),
            });
        });
        Ok(future.await)
    }
}
