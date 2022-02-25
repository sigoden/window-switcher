#![windows_subsystem = "windows"]

#[macro_use]
extern crate lazy_static;

use std::{collections::HashMap, ptr::null_mut, sync::Mutex};
use windows::{
    Win32::Foundation::{BOOL, HWND, LPARAM, PWSTR},
    Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
        SetForegroundWindow, MSG,
    },
    Win32::UI::{
        Input::KeyboardAndMouse::{RegisterHotKey, MOD_ALT, MOD_NOREPEAT},
        WindowsAndMessaging::{GetMessageW, WM_HOTKEY},
    },
};

lazy_static! {
    static ref ALL_WINDOWS: Mutex<HashMap<u32, Vec<HWND>>> = Mutex::new(HashMap::new());
}

fn main() {
    unsafe {
        register_hotkey();
        wait_key_event();
    };
}

unsafe fn wait_key_event() {
    let mut msg = MSG::default();
    loop {
        let res = GetMessageW(&mut msg, HWND(0), 0, 0);
        if res.as_bool() {
            if msg.message == WM_HOTKEY {
                switch_next_window();
            }
        } else {
            break;
        }
    }
}

unsafe fn register_hotkey() -> bool {
    let res = RegisterHotKey(HWND(0), 1, MOD_ALT | MOD_NOREPEAT, 0xC0); // alt + `
    res.into()
}

unsafe fn switch_next_window() -> bool {
    ALL_WINDOWS.lock().unwrap().clear();
    if let Err(_) = enum_windows() {
        return false;
    }
    let hwnd = get_next_window();
    if hwnd.is_none() {
        return false;
    }
    let hwnd = hwnd.unwrap();
    inspect_window(hwnd);
    let res = SetForegroundWindow(hwnd);
    res.into()
}

fn enum_windows() -> windows::core::Result<()> {
    unsafe { EnumWindows(Some(enum_window), LPARAM(0)).ok() }
}

extern "system" fn enum_window(hwnd: HWND, _: LPARAM) -> BOOL {
    unsafe {
        let tid = get_window_pid(hwnd);
        if let Ok(mut all_windows) = ALL_WINDOWS.lock() {
            all_windows.entry(tid).or_default().push(hwnd);
            true.into()
        } else {
            false.into()
        }
    }
}

unsafe fn get_next_window() -> Option<HWND> {
    let hwnd = GetForegroundWindow();
    let pid = get_window_pid(hwnd);
    let all_windows = ALL_WINDOWS.lock().ok()?;
    match all_windows.get(&pid) {
        None => None,
        Some(windows) => {
            let len = windows.len();
            if len == 1 {
                return None;
            }
            let index = windows.iter().position(|v| *v == hwnd)?;
            let index = (index + 1) % len;
            Some(windows[index])
        }
    }
}

unsafe fn inspect_window(hwnd: HWND) {
    let tid = get_window_pid(hwnd);
    let title = get_window_title(hwnd).unwrap_or_default();
    println!("{} {}", tid, title);
}

unsafe fn get_window_pid(hwnd: HWND) -> u32 {
    GetWindowThreadProcessId(hwnd, null_mut())
}

unsafe fn get_window_title(hwnd: HWND) -> Option<String> {
    let mut text: [u16; 512] = [0; 512];
    let len = GetWindowTextW(hwnd, PWSTR(text.as_mut_ptr()), text.len() as i32);
    if len == 0 {
        return None;
    }
    let text = String::from_utf16_lossy(&text[..len as usize]);
    if text.is_empty() {
        return None;
    }
    Some(text)
}
