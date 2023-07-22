use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Weak,
};

use irondash_message_channel::Late;
use windows::{
    core::{implement, ComInterface, IUnknown},
    Win32::{
        Foundation::BOOL,
        UI::{
            Input::KeyboardAndMouse::{
                GetKeyboardLayout, GetKeyboardLayoutList, MapVirtualKeyW, ToUnicodeEx,
                MAPVK_VK_TO_VSC, MAPVK_VSC_TO_VK, VK_CONTROL, VK_MENU, VK_SHIFT, VK_SPACE,
            },
            TextServices::{
                CLSID_TF_InputProcessorProfiles, ITfInputProcessorProfiles,
                ITfLanguageProfileNotifySink, ITfLanguageProfileNotifySink_Impl, ITfSource, HKL,
                TF_INVALID_COOKIE,
            },
        },
    },
};

use crate::{
    keyboard_layout_manager::{Key, KeyboardLayout, KeyboardLayoutDelegate},
    log::OkLog,
};

use super::common::create_instance;

pub struct PlatformKeyboardLayout {
    source: Late<ITfSource>,
    cookie: Cell<u32>,
    cached_layout: RefCell<HashMap<isize, KeyboardLayout>>,
    delegate: Weak<dyn KeyboardLayoutDelegate>,
}

include!(concat!(env!("OUT_DIR"), "/generated_keyboard_map.rs"));

impl PlatformKeyboardLayout {
    pub fn new(delegate: Weak<dyn KeyboardLayoutDelegate>) -> Self {
        Self {
            source: Late::new(),
            cookie: Cell::new(TF_INVALID_COOKIE),
            cached_layout: RefCell::new(HashMap::new()),
            delegate,
        }
    }

    pub fn get_current_layout(&self) -> Option<KeyboardLayout> {
        let current = unsafe { GetKeyboardLayout(0) };
        Some(
            self.cached_layout
                .borrow_mut()
                .entry(current.0)
                .or_insert_with(|| self.create_keyboard_layout())
                .clone(),
        )
    }

    fn create_keyboard_layout(&self) -> KeyboardLayout {
        let key_map = get_key_map();

        let layout = unsafe { self.get_keyboard_layout() };
        let keys: Vec<Key> = unsafe {
            key_map
                .iter()
                .map(|a| self.key_from_entry(a, layout))
                .collect()
        };

        KeyboardLayout { keys }
    }

    unsafe fn get_keyboard_layout(&self) -> HKL {
        let current = GetKeyboardLayout(0);

        // If current layout is ascii capable but with numbers having diacritics, accept that
        if self.is_ascii_capable(current, false) {
            return current;
        }

        let cnt = GetKeyboardLayoutList(Some(&mut []));
        let mut vec: Vec<HKL> = vec![HKL(0); cnt as usize];
        GetKeyboardLayoutList(Some(&mut vec));

        // if choosing from list, prefer layout that has actual numbers
        for l in &vec {
            if self.is_ascii_capable(*l, true) {
                return *l;
            }
        }
        for l in &vec {
            if self.is_ascii_capable(*l, false) {
                return *l;
            }
        }

        current
    }

    unsafe fn is_ascii_capable(&self, hkl: HKL, including_numbers: bool) -> bool {
        // A .. Z
        for vc in 0x41..0x5A {
            let sc = MapVirtualKeyW(vc, MAPVK_VK_TO_VSC);
            let char = Self::get_character(vc, sc, false, false, hkl);
            match char {
                Some(char) => {
                    if char < 'a' as u16 || char > 'z' as u16 {
                        return false;
                    }
                }
                None => return false,
            }
        }
        if including_numbers {
            // 0 .. 9
            for vc in 0x30..0x39 {
                let sc = MapVirtualKeyW(vc, MAPVK_VK_TO_VSC);
                let char = Self::get_character(vc, sc, false, false, hkl);
                match char {
                    Some(char) => {
                        if char < '0' as u16 || char > '9' as u16 {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
        }
        true
    }

    unsafe fn get_character(vc: u32, sc: u32, shift: bool, alt: bool, hkl: HKL) -> Option<u16> {
        let key_state = &mut [0u8; 256];
        let buf = &mut [0u16, 10];

        if shift {
            key_state[VK_SHIFT.0 as usize] = 128;
        }

        if alt {
            key_state[VK_CONTROL.0 as usize] = 128;
            key_state[VK_MENU.0 as usize] = 128;
        }

        // According to documentation, since Windows 10 version 1607 if bit 2 is
        // set the call will not change keyboard state.
        let flags = 0x04;

        let res = ToUnicodeEx(vc, sc, key_state, buf, flags, hkl);

        // Clear keyboard state
        loop {
            let key_state = &mut [0u8; 256];
            let buf = &mut [0u16, 10];
            let res = ToUnicodeEx(
                VK_SPACE.0 as u32,
                MapVirtualKeyW(VK_SPACE.0 as u32, MAPVK_VK_TO_VSC),
                key_state,
                buf,
                flags,
                hkl,
            );
            if res >= 0 {
                break;
            }
        }

        if res > 0 && buf[0] >= 0x20 {
            Some(buf[0])
        } else {
            None
        }
    }

    unsafe fn key_from_entry(&self, entry: &KeyMapEntry, hkl: HKL) -> Key {
        let mut key = Key {
            platform: entry.platform,
            physical: entry.physical,
            logical: entry.logical,
            logical_shift: None,
            logical_alt: None,
            logical_alt_shift: None,
            logical_meta: None,
        };

        let virtual_code = MapVirtualKeyW(entry.platform as u32, MAPVK_VSC_TO_VK);

        let character = Self::get_character(virtual_code, entry.platform as u32, false, false, hkl);

        // This is a printable character
        if let Some(character) = character {
            key.logical = Some(character as i64);

            key.logical_shift =
                Self::get_character(virtual_code, entry.platform as u32, true, false, hkl)
                    .map(|i| i as i64);

            key.logical_alt =
                Self::get_character(virtual_code, entry.platform as u32, false, true, hkl)
                    .map(|i| i as i64);

            key.logical_alt_shift =
                Self::get_character(virtual_code, entry.platform as u32, true, true, hkl)
                    .map(|i| i as i64);

            // println!(
            //     "{:?} - {:?} {:?} {:?} {:?}",
            //     virtual_code,
            //     key.logical
            //         .map(|c| String::from_utf16_lossy(&[c as u16]))
            //         .unwrap_or("--".into()),
            //     key.logical_shift
            //         .map(|c| String::from_utf16_lossy(&[c as u16]))
            //         .unwrap_or("--".into()),
            //     key.logical_alt
            //         .map(|c| String::from_utf16_lossy(&[c as u16]))
            //         .unwrap_or("--".into()),
            //     key.logical_alt_shift
            //         .map(|c| String::from_utf16_lossy(&[c as u16]))
            //         .unwrap_or("--".into())
            // );
        }

        key
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformKeyboardLayout>) {
        let profiles: ITfInputProcessorProfiles =
            create_instance(&CLSID_TF_InputProcessorProfiles).unwrap();
        let source = profiles.cast::<ITfSource>().unwrap();
        let sink: ITfLanguageProfileNotifySink = LanguageProfileNotifySink::new(weak).into();

        unsafe {
            let cookie = source
                .AdviseSink(
                    &ITfLanguageProfileNotifySink::IID,
                    &sink.cast::<IUnknown>().unwrap(),
                )
                .ok_log()
                .unwrap_or(0);
            self.cookie.set(cookie);
        }

        self.source.set(source);
    }

    fn keyboard_layout_changed(&self) {
        if let Some(delegate) = self.delegate.upgrade() {
            delegate.keyboard_map_did_change();
        }
    }
}

impl Drop for PlatformKeyboardLayout {
    fn drop(&mut self) {
        if self.cookie.get() != TF_INVALID_COOKIE {
            unsafe {
                self.source.UnadviseSink(self.cookie.get()).ok_log();
            }
        }
    }
}

//
// Implementation of ITfLanguageProfileNotifySink
//

#[implement(ITfLanguageProfileNotifySink)]
struct LanguageProfileNotifySink {
    target: Weak<PlatformKeyboardLayout>,
}

impl LanguageProfileNotifySink {
    fn new(target: Weak<PlatformKeyboardLayout>) -> Self {
        Self { target }
    }
}

#[allow(non_snake_case)]
impl ITfLanguageProfileNotifySink_Impl for LanguageProfileNotifySink {
    fn OnLanguageChange(&self, _langid: u16) -> windows::core::Result<BOOL> {
        Ok(true.into())
    }

    fn OnLanguageChanged(&self) -> windows::core::Result<()> {
        if let Some(target) = self.target.upgrade() {
            target.keyboard_layout_changed();
        }
        Ok(())
    }
}
