use std::{
    cell::{Cell, RefCell},
    os::raw::c_ulong,
    rc::Weak,
};

use gdk::{
    glib::translate::from_glib_none, prelude::StaticType, Display, Event, EventKey, Keymap,
    KeymapKey,
};
use gtk::Widget;
use irondash_message_channel::Late;

use crate::{
    keyboard_layout_manager::{Key, KeyboardLayout, KeyboardLayoutDelegate},
    log::OkLog,
};

use super::signal::Signal;

pub struct PlatformKeyboardLayout {
    current_layout: RefCell<Option<KeyboardLayout>>,
    current_group: Cell<u8>,
    delegate: Weak<dyn KeyboardLayoutDelegate>,
    key_press_hook: Late<c_ulong>,
}

include!(concat!(env!("OUT_DIR"), "/generated_keyboard_map.rs"));

fn lookup_key(keymap: &Keymap, key: &KeymapKey) -> Option<i64> {
    // Weird behavior, on SVK keyboard enter returns 'a' and left control returns 'A'.
    if key.keycode() == 36 || key.keycode() == 37 {
        return None;
    }
    let res = keymap.lookup_key(key)?.to_unicode()? as i64;
    if res < 0x20 {
        // ignore control characters
        return None;
    }
    Some(res)
}

impl PlatformKeyboardLayout {
    pub fn new(delegate: Weak<dyn KeyboardLayoutDelegate>) -> Self {
        unsafe { gtk::set_initialized() };
        Self {
            current_group: Cell::new(0),
            current_layout: RefCell::new(None),
            delegate,
            key_press_hook: Late::new(),
        }
    }

    pub fn get_current_layout(&self) -> Option<KeyboardLayout> {
        Some(
            self.current_layout
                .borrow_mut()
                .get_or_insert_with(|| self.create_keyboard_layout())
                .clone(),
        )
    }

    fn create_keyboard_layout(&self) -> KeyboardLayout {
        let key_map = get_key_map();
        if let Some(display) = Display::default() {
            if let Some(keymap) = Keymap::for_display(&display) {
                let group = self.get_group(&keymap);
                let keys: Vec<Key> = key_map
                    .iter()
                    .map(|a| self.key_from_entry(a, &keymap, group))
                    .collect();
                return KeyboardLayout { keys };
            }
        }

        Self::fallback_map(&key_map)
    }

    fn get_group(&self, keymap: &Keymap) -> u8 {
        // If current layout is ascii capable but with numbers having diacritics, accept that
        if self.is_ascii_capable(keymap, false, self.current_group.get()) {
            return self.current_group.get();
        }

        // if choosing from list, prefer layout that has actual numbers
        for group in 0..3 {
            if self.is_ascii_capable(keymap, true, group) {
                return group;
            }
        }

        for group in 0..3 {
            if self.is_ascii_capable(keymap, false, group) {
                return group;
            }
        }

        self.current_group.get()
    }

    fn is_ascii(&self, keymap: &Keymap, group: u8, code: u32) -> bool {
        let key = lookup_key(
            keymap,
            &Self::create_key(gdk::ffi::GdkKeymapKey {
                keycode: code,
                group: group as _,
                level: 0,
            }),
        );
        if let Some(key) = key {
            if key < 256 {
                let char = key as u8 as char;
                return char.is_ascii_lowercase() || char.is_ascii_digit();
            }
        }
        false
    }

    fn is_ascii_capable(&self, keymap: &Keymap, including_numbers: bool, group: u8) -> bool {
        // Q - P
        for key in 24..33 {
            if !self.is_ascii(keymap, group, key) {
                return false;
            }
        }
        // A - L
        for key in 38..46 {
            if !self.is_ascii(keymap, group, key) {
                return false;
            }
        }
        // Z - M
        for key in 52..58 {
            if !self.is_ascii(keymap, group, key) {
                return false;
            }
        }

        if including_numbers {
            // 0 - 1
            for key in 10..19 {
                if !self.is_ascii(keymap, group, key) {
                    return false;
                }
            }
        }

        true
    }

    fn create_key(key: gdk::ffi::GdkKeymapKey) -> gdk::KeymapKey {
        unsafe { from_glib_none(&key as *const _) }
    }

    fn key_from_entry(&self, entry: &KeyMapEntry, keymap: &Keymap, group: u8) -> Key {
        let key = lookup_key(
            keymap,
            &Self::create_key(gdk::ffi::GdkKeymapKey {
                keycode: entry.platform as u32,
                group: group as _,
                level: 0,
            }),
        );

        let key_shift = if let Some(_key) = key {
            lookup_key(
                keymap,
                &Self::create_key(gdk::ffi::GdkKeymapKey {
                    keycode: entry.platform as u32,
                    group: group as _,
                    level: 1,
                }),
            )
        } else {
            None
        };

        Key {
            platform: entry.platform,
            physical: entry.physical,
            logical: key.or(entry.logical),
            logical_shift: key_shift,
            logical_alt: None,
            logical_alt_shift: None,
            logical_meta: None,
        }
    }

    fn fallback_map(keys: &[KeyMapEntry]) -> KeyboardLayout {
        KeyboardLayout {
            keys: keys.iter().map(Self::fallback_key_from_entry).collect(),
        }
    }

    fn fallback_key_from_entry(entry: &KeyMapEntry) -> Key {
        Key {
            platform: entry.platform,
            physical: entry.physical,
            logical: entry.fallback,
            logical_shift: entry.fallback.and_then(Self::shift_key),
            logical_alt: None,
            logical_alt_shift: None,
            logical_meta: None,
        }
    }

    fn shift_key(key: i64) -> Option<i64> {
        if key < 256 {
            Some(Self::_shift_key(key as u8 as char) as u8 as i64)
        } else {
            None
        }
    }

    // According to US layout
    fn _shift_key(key: char) -> char {
        match key {
            '`' => '~',
            '1' => '!',
            '2' => '@',
            '3' => '#',
            '4' => '$',
            '5' => '%',
            '6' => '^',
            '7' => '&',
            '8' => '*',
            '9' => '(',
            '0' => ')',
            '-' => '_',
            '=' => '+',
            '[' => '{',
            ']' => '}',
            '\\' => '|',
            ';' => ':',
            '\'' => '"',
            ',' => '<',
            '.' => '>',
            '/' => '?',
            c => {
                if c.is_ascii_lowercase() {
                    let delta = b'A' as i32 - b'a' as i32;
                    (c as u8 as i32 + delta) as u8 as char
                } else {
                    c
                }
            }
        }
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformKeyboardLayout>) {
        if let Some(signal) = Signal::lookup("key-press-event", Widget::static_type()) {
            let hook = signal.add_emission_hook(move |_, values| {
                if let Some(this) = weak.clone().upgrade() {
                    if let Some(event) = values[1].get::<Event>().ok_log() {
                        this.on_key_event(&event);
                    }
                }
                true
            });
            self.key_press_hook.set(hook);
        }
    }

    pub(crate) fn on_key_event(&self, event: &Event) {
        if let Some(event) = event.downcast_ref::<EventKey>() {
            let group = event.group();
            if group != self.current_group.get() {
                self.current_group.set(group);
                self.on_layout_changed();
            }
        }
    }

    fn on_layout_changed(&self) {
        self.current_layout.borrow_mut().take();
        if let Some(delegate) = self.delegate.upgrade() {
            delegate.keyboard_map_did_change();
        }
    }
}

impl Drop for PlatformKeyboardLayout {
    fn drop(&mut self) {
        if let Some(signal) = Signal::lookup("key-press-event", Widget::static_type()) {
            signal.remove_emission_hook(*self.key_press_hook);
        }
    }
}
