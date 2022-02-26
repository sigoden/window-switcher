use std::{collections::HashMap, ptr::null_mut, sync::Mutex};
use windows::{
    Win32::Foundation::{BOOL, HWND, LPARAM},
    Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
    },
};

lazy_static! {
    static ref ALL_WINDOWS: Mutex<HashMap<u32, Vec<HWND>>> = Mutex::new(HashMap::new());
}

pub fn switch_next_window() -> bool {
    unsafe {
        ALL_WINDOWS.lock().unwrap().clear();
        if enum_windows().is_err() {
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

fn enum_windows() -> windows::core::Result<()> {
    unsafe { EnumWindows(Some(enum_window), LPARAM(0)).ok() }
}

fn get_next_window() -> Option<HWND> {
    let (hwnd, pid) = unsafe {
        let hwnd = GetForegroundWindow();
        let pid = get_window_pid(hwnd);
        (hwnd, pid)
    };
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
