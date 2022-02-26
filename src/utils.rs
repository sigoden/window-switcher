use windows::Win32::Foundation::{HWND, PWSTR};
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK, MESSAGEBOX_STYLE};

#[allow(unused)]
pub fn info_msg(caption: &str) {
    msgbox("Info", caption, MB_OK);
}

#[allow(unused)]
pub fn error_msg(caption: &str) {
    msgbox("Error", caption, MB_OK | MB_ICONERROR);
}

pub fn msgbox(text: &str, caption: &str, style: MESSAGEBOX_STYLE) {
    let text = wchar_ptr(text);
    let caption = wchar_ptr(caption);
    unsafe {
        MessageBoxW(HWND(0), text, caption, style);
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
