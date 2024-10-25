use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use windows::core::PWSTR;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, MAX_PATH, POINT, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Console::{AllocConsole, FreeConsole, GetConsoleWindow};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_INFORMATION,
    PROCESS_VM_READ,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetCursorPos, GetForegroundWindow, GetWindow, GetWindowLongPtrW,
    GetWindowPlacement, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    SetForegroundWindow, SetWindowPos, ShowWindow, GWL_EXSTYLE, GWL_USERDATA, GW_OWNER,
    SWP_NOZORDER, SW_RESTORE, WINDOWPLACEMENT, WS_EX_TOPMOST,
};

use std::path::PathBuf;
use std::{ffi::c_void, mem::size_of};

pub fn is_iconic_window(hwnd: HWND) -> bool {
    unsafe { IsIconic(hwnd) }.as_bool()
}

pub fn is_visible_window(hwnd: HWND) -> bool {
    let ret = unsafe { IsWindowVisible(hwnd) };
    ret.as_bool()
}

pub fn is_topmost_window(hwnd: HWND) -> bool {
    let ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    ex_style & WS_EX_TOPMOST.0 != 0
}

pub fn is_cloaked_window(hwnd: HWND) -> bool {
    let mut cloaked = 0u32;
    let _ = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut c_void,
            size_of::<u32>() as u32,
        )
    };
    cloaked != 0
}

pub fn is_small_window(hwnd: HWND) -> bool {
    let (width, height) = get_window_size(hwnd);
    width < 120 || height < 90
}

pub fn get_moinitor_rect() -> RECT {
    unsafe {
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..MONITORINFO::default()
        };
        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);

        let hmonitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        let _ = GetMonitorInfoW(hmonitor, &mut mi);
        mi.rcMonitor
    }
}

pub fn get_window_size(hwnd: HWND) -> (i32, i32) {
    let mut placement = WINDOWPLACEMENT::default();
    let _ = unsafe { GetWindowPlacement(hwnd, &mut placement) };
    let rect = placement.rcNormalPosition;
    ((rect.right - rect.left), (rect.bottom - rect.top))
}

pub fn get_exe_folder() -> Result<PathBuf> {
    let path =
        std::env::current_exe().map_err(|err| anyhow!("Failed to get binary path, {err}"))?;
    path.parent()
        .ok_or_else(|| anyhow!("Failed to get binary folder"))
        .map(|v| v.to_path_buf())
}

pub fn get_exe_path() -> Vec<u16> {
    let mut path = vec![0u16; MAX_PATH as _];
    let size = unsafe { GetModuleFileNameW(None, &mut path) } as usize;
    path[..size].to_vec()
}

pub fn get_window_pid(hwnd: HWND) -> u32 {
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid as *mut u32)) };
    pid
}

pub fn get_module_path(pid: u32) -> Option<String> {
    let handle =
        match unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, None, pid) } {
            Ok(v) => v,
            Err(_) => return None,
        };
    let mut len: u32 = MAX_PATH;
    let mut name = vec![0u16; len as usize];
    let ret = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(name.as_mut_ptr()),
            &mut len,
        )
    };
    if ret.is_err() || len == 0 {
        return None;
    }
    unsafe { name.set_len(len as usize) };
    let module_path = String::from_utf16_lossy(&name);
    if module_path.is_empty() {
        return None;
    }
    Some(module_path)
}

pub fn get_window_exe(hwnd: HWND) -> Option<String> {
    let pid = get_window_pid(hwnd);
    if pid == 0 {
        return None;
    }
    let module_path = get_module_path(pid)?;
    module_path.split('\\').map(|v| v.to_string()).last()
}

pub fn set_foreground_window(hwnd: HWND) {
    unsafe {
        if is_iconic_window(hwnd) {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        if hwnd == get_foreground_window() {
            return;
        }
        if SetForegroundWindow(hwnd).ok().is_err() {
            let _ = AllocConsole();
            let hwnd_console = GetConsoleWindow();
            let _ = SetWindowPos(hwnd_console, None, 0, 0, 0, 0, SWP_NOZORDER);
            let _ = FreeConsole();
            let _ = SetForegroundWindow(hwnd);
        }
    };
}

pub fn get_foreground_window() -> HWND {
    unsafe { GetForegroundWindow() }
}

pub fn get_window_title(hwnd: HWND) -> String {
    let mut buf = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_slice()) };
    if len == 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..len as usize])
}

pub fn get_owner_window(hwnd: HWND) -> HWND {
    unsafe { GetWindow(hwnd, GW_OWNER) }.unwrap_or_default()
}

#[cfg(target_arch = "x86")]
pub fn get_window_user_data(hwnd: HWND) -> i32 {
    unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowLongW(hwnd, GWL_USERDATA) }
}
#[cfg(not(target_arch = "x86"))]
pub fn get_window_user_data(hwnd: HWND) -> isize {
    unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd, GWL_USERDATA) }
}

#[cfg(target_arch = "x86")]
pub fn set_window_user_data(hwnd: HWND, ptr: i32) -> i32 {
    unsafe { windows::Win32::UI::WindowsAndMessaging::SetWindowLongW(hwnd, GWL_USERDATA, ptr) }
}

#[cfg(not(target_arch = "x86"))]
pub fn set_window_user_data(hwnd: HWND, ptr: isize) -> isize {
    unsafe { windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(hwnd, GWL_USERDATA, ptr) }
}

/// Lists available windows
///
/// Duo to the limitation of `OpenProcess`, this function will not list `Task Manager`
/// and others which are running as administrator if `Switcher` is not `running as administrator`.
pub fn list_windows(ignore_minimal: bool) -> Result<IndexMap<String, Vec<(HWND, String)>>> {
    let mut result: IndexMap<String, Vec<(HWND, String)>> = IndexMap::new();
    let mut hwnds: Vec<HWND> = Default::default();
    unsafe { EnumWindows(Some(enum_window), LPARAM(&mut hwnds as *mut _ as isize)) }
        .map_err(|e| anyhow!("Fail to get windows {}", e))?;
    let mut valid_hwnds = vec![];
    let mut owner_hwnds = vec![];
    for hwnd in hwnds.iter().cloned() {
        let mut valid = is_visible_window(hwnd)
            && !is_cloaked_window(hwnd)
            && !is_topmost_window(hwnd)
            && !is_small_window(hwnd);
        if valid && ignore_minimal && is_iconic_window(hwnd) {
            valid = false;
        }
        if valid {
            let title = get_window_title(hwnd);
            if !title.is_empty() && title != "Program Manager" {
                valid_hwnds.push((hwnd, title))
            }
        }
        owner_hwnds.push(get_owner_window(hwnd))
    }
    for (hwnd, title) in valid_hwnds.into_iter() {
        let mut rs_module_path = "".to_string();
        if let Some(module_path) = get_module_path(get_window_pid(hwnd)) {
            rs_module_path = module_path;
        }
        if !rs_module_path.is_empty()
            && rs_module_path != "C:\\Windows\\System32\\ApplicationFrameHost.exe"
        {
            result
                .entry(rs_module_path)
                .or_default()
                .push((hwnd, title));
            continue;
        }
        if let Some((i, _)) = owner_hwnds.iter().enumerate().find(|(_, v)| **v == hwnd) {
            if let Some(module_path) = get_module_path(get_window_pid(hwnds[i])) {
                rs_module_path = module_path;
            }
        }
        if !rs_module_path.is_empty()
            && rs_module_path != "C:\\Windows\\System32\\ApplicationFrameHost.exe"
        {
            result
                .entry(rs_module_path)
                .or_default()
                .push((hwnd, title));
        }
    }
    debug!("list windows {:?}", result);
    Ok(result)
}

extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows: &mut Vec<HWND> = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
    windows.push(hwnd);
    BOOL(1)
}
