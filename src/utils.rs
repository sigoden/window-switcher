use windows::Win32::Foundation::{HWND, PWSTR};
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, MOD_ALT, MOD_NOREPEAT};

#[cfg(debug_assertions)]
mod debug {
    use std::ffi::CString;

    use windows::Win32::Foundation::PSTR;
    use windows::Win32::System::Diagnostics::Debug::OutputDebugStringA;
    pub fn output_debug(text: &str) {
        let data = CString::new(text).unwrap();
        unsafe { OutputDebugStringA(PSTR(data.as_ptr() as *const u8)) };
    }
}
#[cfg(debug_assertions)]
pub use debug::output_debug;
#[cfg(not(debug_assertions))]
pub fn output_debug(_text: &str) {}

pub fn register_hotkey() {
    let ret = unsafe { RegisterHotKey(HWND(0), 1, MOD_ALT | MOD_NOREPEAT, 0xC0) }; // alt + `
    if !ret.as_bool() {
        output_debug("Utils: Fail to register hotkey");
    }
}

pub fn wchar_array(string: &str, dst: &mut [u16]) {
    let mut s = string.encode_utf16().collect::<Vec<_>>();

    // Truncate utf16 array to fit in the buffer with null terminator
    s.truncate(dst.len() - 1);

    dst[..s.len()].copy_from_slice(s.as_slice());

    // Null terminator
    dst[s.len()] = 0;
}

pub fn wchar_ptr(string: &str) -> PWSTR {
    let w = wchar(string);
    PWSTR(w.as_ptr())
}

pub fn wchar(string: &str) -> Vec<u16> {
    format!("{}\0", string).encode_utf16().collect::<Vec<_>>()
}
