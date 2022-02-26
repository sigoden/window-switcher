use windows::{
    Win32::Foundation::HWND,
    Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, MOD_ALT, MOD_NOREPEAT},
};

pub fn register_hotkey() {
    unsafe { RegisterHotKey(HWND(0), 1, MOD_ALT | MOD_NOREPEAT, 0xC0) }; // alt + `
}
