#![windows_subsystem = "windows"]

mod trayicon;

#[macro_use]
extern crate lazy_static;

use std::{collections::HashMap, ptr::null_mut, sync::Mutex};
use trayicon::TrayIcon;
use windows::{
    Win32::Foundation::{BOOL, HWND, LPARAM},
    Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, EnumWindows, GetForegroundWindow, GetWindowThreadProcessId,
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
    TrayIcon::create();
    register_hotkey();
    eventloop();
}

fn eventloop() {
    unsafe {
        let mut msg = MSG::default();
        loop {
            let res = GetMessageW(&mut msg, HWND(0), 0, 0);
            if res.as_bool() {
                if msg.message == WM_HOTKEY {
                    switch_next_window();
                }
                DispatchMessageW(&msg);
            } else {
                break;
            }
        }
    }
}

fn register_hotkey() -> bool {
    let res = unsafe { RegisterHotKey(HWND(0), 1, MOD_ALT | MOD_NOREPEAT, 0xC0) }; // alt + `
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

unsafe fn get_window_pid(hwnd: HWND) -> u32 {
    GetWindowThreadProcessId(hwnd, null_mut())
}
