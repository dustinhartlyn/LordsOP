//! Display startup errors in release builds, which have no Windows console.

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};


    fn wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s).encode_wide().chain([0]).collect()
    }

    pub fn error(title: &str, text: &str) {
        let (t, c) = (wide(text), wide(title));
        unsafe { MessageBoxW(0, t.as_ptr(), c.as_ptr(), MB_OK | MB_ICONERROR) };
    }

}

#[cfg(not(windows))]
mod imp {
    pub fn error(title: &str, text: &str) {
        eprintln!("{title}: {text}");
    }
}

pub(crate) use imp::*;
