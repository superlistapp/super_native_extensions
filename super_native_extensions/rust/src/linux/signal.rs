use std::{
    ffi::CString,
    mem::ManuallyDrop,
    os::raw::{c_uint, c_ulong, c_void},
    slice,
};

use gdk::glib::{
    translate::{from_glib_none, IntoGlib},
    Type, Value,
};
use glib_sys::{gboolean, gpointer};
use gobject_sys::{
    g_signal_add_emission_hook, g_signal_lookup, g_signal_remove_emission_hook,
    GSignalInvocationHint, GValue,
};

pub struct Signal(c_uint);

impl Signal {
    pub fn lookup(name: &str, ty: Type) -> Option<Self> {
        let name = CString::new(name).unwrap();
        let signal = unsafe { g_signal_lookup(name.as_ptr(), ty.into_glib()) };
        if signal == 0 {
            None
        } else {
            Some(Self(signal))
        }
    }

    extern "C" fn on_hook(
        hint: *mut GSignalInvocationHint,
        a: c_uint,
        v: *const GValue,
        p: gpointer,
    ) -> gboolean {
        let b: Box<Box<dyn Fn(&GSignalInvocationHint, &[Value]) -> bool>> =
            unsafe { Box::from_raw(p as *mut _) };
        let b = ManuallyDrop::new(b);
        let values = unsafe { slice::from_raw_parts(v, a as usize) };
        let values: Vec<Value> = values
            .iter()
            .map(|v| unsafe { from_glib_none(v as *const _) })
            .collect();
        let hint = unsafe { &*hint };
        b(hint, &values).into_glib()
    }

    extern "C" fn on_delete(p: gpointer) {
        let _b: Box<Box<dyn Fn(&GSignalInvocationHint, &[Value]) -> bool>> =
            unsafe { Box::from_raw(p as *mut _) };
    }

    pub fn add_emission_hook<F>(&self, f: F) -> c_ulong
    where
        F: Fn(&GSignalInvocationHint, &[Value]) -> bool + 'static,
    {
        let b: Box<dyn Fn(&GSignalInvocationHint, &[Value]) -> bool> = Box::new(f);
        let b = Box::new(b);

        unsafe {
            g_signal_add_emission_hook(
                self.0,
                0,
                Some(Self::on_hook),
                Box::into_raw(b) as *mut c_void,
                Some(Self::on_delete),
            )
        }
    }

    pub fn remove_emission_hook(&self, hook_id: c_ulong) {
        unsafe {
            g_signal_remove_emission_hook(self.0, hook_id);
        }
    }
}
