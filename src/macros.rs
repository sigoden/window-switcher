use std::ffi::CString;
use std::fmt::Display;

use windows::core::PCSTR;
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxA, MB_ICONERROR, MB_OK};

pub fn message_box<T: Display>(text: T) {
    let lp_text = CString::new(text.to_string()).unwrap();
    let lp_caption = CString::new("Windows Switcher Error").unwrap();
    unsafe {
        MessageBoxA(
            None,
            PCSTR(lp_text.as_ptr() as *const u8),
            PCSTR(lp_caption.as_ptr() as *const u8),
            MB_OK | MB_ICONERROR,
        )
    };
}

#[macro_export]
macro_rules! alert {
    ($($arg:tt)*) => {
        $crate::macros::message_box(format!($($arg)*))
    };
}
