use std::ffi::CString;
use std::fmt::Display;

use windows::core::PCSTR;
use windows::Win32::System::Diagnostics::Debug::OutputDebugStringA;
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxA, MB_ICONERROR, MB_OK};

pub fn output_debug<T: Display>(text: T) {
    let data = CString::new(text.to_string()).unwrap();
    unsafe { OutputDebugStringA(PCSTR(data.as_ptr() as *const u8)) };
}

pub fn message_box<T: Display>(text: T) {
    let lp_text = CString::new(text.to_string()).unwrap();
    let lp_caption = CString::new("Error").unwrap();
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
macro_rules! error {
	($($arg:tt)*) => {
        $crate::macros::output_debug(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! debug {
	($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        $crate::macros::output_debug(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! alert {
    ($($arg:tt)*) => {
        $crate::macros::message_box(format!($($arg)*))
    };
}
