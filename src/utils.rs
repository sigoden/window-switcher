//! win32 utils

use anyhow::{anyhow, Result};
use windows::Win32::Foundation::{BOOL, HWND, PWSTR};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_INFORMATION,
    PROCESS_VM_READ,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, RegisterHotKey, UnregisterHotKey, MOD_NOREPEAT, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowPlacement, GetWindowTextW, GetWindowThreadProcessId,
    IsWindowVisible, SetForegroundWindow, ShowWindow, SHOW_WINDOW_CMD, SW_RESTORE,
    SW_SHOWMINIMIZED, WINDOWPLACEMENT,
};

use crate::HotKeyConfig;

pub fn get_window_pid(hwnd: HWND) -> u32 {
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid as *mut u32) };
    pid
}

pub fn is_window_visible(hwnd: HWND) -> bool {
    let ret = unsafe { IsWindowVisible(hwnd) };
    ret.as_bool()
}

pub fn get_window_placement(hwnd: HWND) -> SHOW_WINDOW_CMD {
    let mut placement = WINDOWPLACEMENT::default();
    unsafe { GetWindowPlacement(hwnd, &mut placement) };
    placement.showCmd
}

pub fn is_window_minimized(hwnd: HWND) -> bool {
    let placement = get_window_placement(hwnd);
    placement == SW_SHOWMINIMIZED
}

pub fn get_window_exe_name(hwnd: HWND) -> String {
    let module_path = get_window_module_path(hwnd);
    module_path
        .split('\\')
        .last()
        .unwrap_or_default()
        .to_lowercase()
}

pub fn get_window_module_path(hwnd: HWND) -> String {
    get_module_path(get_window_pid(hwnd))
}

pub fn get_foreground_window() -> HWND {
    unsafe { GetForegroundWindow() }
}

pub fn get_window_title(hwnd: HWND) -> String {
    let buf = [0u16; 512];
    let len = buf.len();
    let len = unsafe { GetWindowTextW(hwnd, PWSTR(buf.as_ptr()), len as i32) };
    if len == 0 {
        return String::default();
    }
    String::from_utf16_lossy(&buf[..len as usize])
}

pub fn get_module_path(pid: u32) -> String {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, BOOL(0), pid) };
    if handle.is_invalid() {
        return String::default();
    }
    let mut len: u32 = 1024;
    let mut name = vec![0u16; len as usize];
    let ret = unsafe {
        QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(name.as_ptr()), &mut len)
    };
    if !ret.as_bool() || len == 0 {
        return String::default();
    }
    unsafe { name.set_len(len as usize) };
    String::from_utf16_lossy(&name)
}

pub fn switch_to(hwnd: HWND) -> Result<()> {
    if get_window_placement(hwnd) == SW_SHOWMINIMIZED {
        unsafe { ShowWindow(hwnd, SW_RESTORE) }
            .ok()
            .map_err(|e| anyhow!("Fail to show window, {}", e))?;
    }
    unsafe { SetForegroundWindow(hwnd) }
        .ok()
        .map_err(|e| anyhow!("Fail to set window to foreground, {}", e))?;

    Ok(())
}

pub fn detect_key_down(vk: VIRTUAL_KEY) -> bool {
    (unsafe { GetKeyState(vk.0.into()) }) < 0
}

pub fn register_hotkey(hwnd: HWND, id: usize, hotkey: &HotKeyConfig) -> Result<()> {
    unsafe {
        RegisterHotKey(
            hwnd,
            id as i32,
            hotkey.modifier() | MOD_NOREPEAT,
            hotkey.code as u32,
        )
    }
    .ok()
    .map_err(|e| anyhow!("Fail to register hotkey {}, {}", id, e))
}

pub fn unregister_hotkey(hwnd: HWND, id: usize) -> Result<()> {
    if id == 0 {
        return Ok(());
    }
    unsafe { UnregisterHotKey(hwnd, id as i32) }
        .ok()
        .map_err(|e| anyhow!("Fail to unregister hotkey {}, {}", id, e))
}
