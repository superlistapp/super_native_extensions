use windows::Win32::System::Ole::{OleInitialize, OleUninitialize};

pub struct OleInitializer {
    need_uninit: bool,
}

impl OleInitializer {
    pub fn new() -> OleInitializer {
        let res = unsafe { OleInitialize(None) };
        OleInitializer {
            need_uninit: res.is_ok(),
        }
    }
}

impl Drop for OleInitializer {
    fn drop(&mut self) {
        if self.need_uninit {
            unsafe {
                OleUninitialize();
            }
        }
    }
}
