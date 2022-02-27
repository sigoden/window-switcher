use anyhow::{anyhow, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Mutex,
};
use windows::{
    Win32::Foundation::{BOOL, HWND, LPARAM, PWSTR},
    Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_INFORMATION,
        PROCESS_VM_READ,
    },
    Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowThreadProcessId,
        IsWindowVisible, SetForegroundWindow,
    },
};

lazy_static! {
    static ref ALL_WINDOWS: Mutex<BTreeMap<Vec<u16>, BTreeSet<isize>>> =
        Mutex::new(BTreeMap::new());
}

pub fn switch_next_window() -> Result<bool> {
    ALL_WINDOWS
        .lock()
        .map_err(|_| anyhow!("Fail to unlock ALL_WINDOWS"))?
        .clear();
    enum_windows()?;
    let hwnd = get_next_window();
    if hwnd.is_none() {
        return Ok(false);
    }
    let hwnd = hwnd.unwrap();
    unsafe { SetForegroundWindow(hwnd) }
        .ok()
        .map_err(|e| anyhow!("Fail to set window to foreground, {:?}", e))?;
    Ok(true)
}

extern "system" fn enum_window(hwnd: HWND, _: LPARAM) -> BOOL {
    if !is_window_visible(hwnd) {
        return true.into();
    }
    if !is_window_have_title(hwnd) {
        return true.into();
    }
    let pid = get_window_pid(hwnd);
    let module_path = get_module_path(pid);
    if module_path.is_empty() {
        return true.into();
    }
    if let Ok(mut all_windows) = ALL_WINDOWS.lock() {
        all_windows.entry(module_path).or_default().insert(hwnd.0);
        true.into()
    } else {
        false.into()
    }
}

fn enum_windows() -> Result<()> {
    unsafe { EnumWindows(Some(enum_window), LPARAM(0)).ok() }
        .map_err(|e| anyhow!("Fail to enum windows {}", e))
}

fn get_next_window() -> Option<HWND> {
    let (hwnd, pid) = unsafe {
        let hwnd = GetForegroundWindow();
        let pid = get_window_pid(hwnd);
        (hwnd, pid)
    };
    let module_path = get_module_path(pid);
    if module_path.is_empty() {
        return None;
    }
    let all_windows = ALL_WINDOWS.lock().ok()?;
    match all_windows.get(&module_path) {
        None => None,
        Some(windows) => {
            let len = windows.len();
            if len == 1 {
                return None;
            }
            let values: Vec<isize> = windows.iter().cloned().collect();
            let index = windows.iter().position(|v| *v == hwnd.0)?;
            let index = (index + 1) % len;
            Some(HWND(values[index]))
        }
    }
}

fn get_window_pid(hwnd: HWND) -> u32 {
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid as *mut u32) };
    pid
}

fn is_window_visible(hwnd: HWND) -> bool {
    let ret = unsafe { IsWindowVisible(hwnd) };
    ret.as_bool()
}

fn is_window_have_title(hwnd: HWND) -> bool {
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    len > 0
}

fn get_module_path(pid: u32) -> Vec<u16> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, BOOL(0), pid) };
    if handle.is_invalid() {
        return Vec::default();
    }
    let mut len: u32 = 1024;
    let mut name = vec![0u16; len as usize];
    let ret = unsafe {
        QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(name.as_ptr()), &mut len)
    };
    if !ret.as_bool() || len == 0 {
        return Vec::default();
    }
    unsafe { name.set_len(len as usize) };
    name
}
