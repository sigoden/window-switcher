use windows::core::PCWSTR;
use windows::w;
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

pub fn message_box(text: &str) {
    let text = text.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr() as _),
            w!("Windows Switcher Error"),
            MB_OK | MB_ICONERROR,
        )
    };
}

#[macro_export]
macro_rules! alert {
    ($($arg:tt)*) => {
        $crate::macros::message_box(&format!($($arg)*))
    };
}
