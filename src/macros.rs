use std::ffi::CString;

use windows::Win32::Foundation::PSTR;
use windows::Win32::System::Diagnostics::Debug::OutputDebugStringA;
pub fn output_debug<T: AsRef<str>>(text: T) {
    let data = CString::new(text.as_ref()).unwrap();
    unsafe { OutputDebugStringA(PSTR(data.as_ptr() as *const u8)) };
}

#[macro_export]
macro_rules! log_error {
    ($msg:literal $(,)?) => {
        $crate::macros::output_debug($msg);
    };
    ($err:expr $(,)?) => {
        $crate::macros::output_debug($err);
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::macros::output_debug(&format!($fmt, $($arg)*));
    };
}

#[macro_export]
macro_rules! log_info {
    ($msg:literal $(,)?) => {
		#[cfg(debug_assertions)]
        $crate::macros::output_debug($msg);
    };
    ($err:expr $(,)?) => {
		#[cfg(debug_assertions)]
        $crate::macros::output_debug($err);
    };
    ($fmt:expr, $($arg:tt)*) => {
		#[cfg(debug_assertions)]
        $crate::macros::output_debug(&format!($fmt, $($arg)*));
    };
}
