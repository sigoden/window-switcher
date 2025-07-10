use crate::utils::is_process_elevated;

use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use std::{ffi::c_void, mem::size_of, path::PathBuf};
use windows::core::{BOOL, PWSTR};
use windows::Win32::{
    Foundation::{HWND, LPARAM, MAX_PATH, POINT, RECT},
    Graphics::{
        Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWM_CLOAKED_SHELL},
        Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST},
    },
    System::{
        LibraryLoader::GetModuleFileNameW,
        Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
    UI::{
        Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_MOUSE},
        WindowsAndMessaging::{
            EnumWindows, GetCursorPos, GetForegroundWindow, GetWindow, GetWindowLongPtrW,
            GetWindowPlacement, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
            SetForegroundWindow, ShowWindow, GWL_EXSTYLE, GWL_STYLE, GWL_USERDATA, GW_OWNER,
            SW_RESTORE, WINDOWPLACEMENT, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_ICONIC, WS_VISIBLE,
        },
    },
};

pub fn get_window_state(hwnd: HWND) -> (bool, bool, bool, bool) {
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32;
    let exstyle = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;

    let is_visible = style & WS_VISIBLE.0 != 0;
    let is_iconic = style & WS_ICONIC.0 != 0;
    let is_tool = exstyle & WS_EX_TOOLWINDOW.0 != 0;
    let is_topmost = exstyle & WS_EX_TOPMOST.0 != 0;

    (is_visible, is_iconic, is_tool, is_topmost)
}

pub fn is_iconic_window(hwnd: HWND) -> bool {
    unsafe { IsIconic(hwnd) }.as_bool()
}

pub fn get_window_cloak_type(hwnd: HWND) -> u32 {
    let mut cloak_type = 0u32;
    let _ = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloak_type as *mut u32 as *mut c_void,
            size_of::<u32>() as u32,
        )
    };
    cloak_type
}

fn is_cloaked_window(hwnd: HWND, only_current_desktop: bool) -> bool {
    let cloak_type = get_window_cloak_type(hwnd);

    if only_current_desktop {
        // Any kind of cloaking counts against a window
        cloak_type != 0
    } else {
        // Windows from other desktops will be cloaked as SHELL, so we treat them
        // as if they are uncloaked. All other cloak types count against the window
        cloak_type | DWM_CLOAKED_SHELL != DWM_CLOAKED_SHELL
    }
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
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
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
    module_path.split('\\').map(|v| v.to_string()).next_back()
}

pub fn set_foreground_window(hwnd: HWND) {
    // ref https://github.com/microsoft/PowerToys/blob/4cb72ee126caf1f720c507f6a1dbe658cd515366/src/modules/fancyzones/FancyZonesLib/WindowUtils.cpp#L191
    unsafe {
        if is_iconic_window(hwnd) {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }

        let input = INPUT {
            r#type: INPUT_MOUSE,
            ..Default::default()
        };

        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);

        let _ = SetForegroundWindow(hwnd);
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
pub fn list_windows(
    ignore_minimal: bool,
    only_current_desktop: bool,
    is_admin: bool,
) -> Result<IndexMap<String, Vec<(HWND, String)>>> {
    let mut result: IndexMap<String, Vec<(HWND, String)>> = IndexMap::new();
    let mut hwnds: Vec<HWND> = Default::default();
    unsafe { EnumWindows(Some(enum_window), LPARAM(&mut hwnds as *mut _ as isize)) }
        .map_err(|e| anyhow!("Fail to get windows {}", e))?;
    let mut valid_hwnds = vec![];
    let mut owner_hwnds = vec![];
    for hwnd in hwnds.iter().cloned() {
        let (is_visible, is_iconic, is_tool, is_topmost) = get_window_state(hwnd);
        let ok = is_visible
            && (if ignore_minimal { !is_iconic } else { true })
            && !is_tool
            && !is_topmost
            && !is_cloaked_window(hwnd, only_current_desktop)
            && !is_small_window(hwnd);
        if ok {
            let title = get_window_title(hwnd);
            if !title.is_empty() && title != "Windows Input Experience" {
                valid_hwnds.push((hwnd, title));
            }
        }
        owner_hwnds.push(get_owner_window(hwnd))
    }
    for (hwnd, title) in valid_hwnds.into_iter() {
        let mut pid = get_window_pid(hwnd);
        let mut module_path = get_module_path(pid).unwrap_or_default();
        if !is_valid_module_path(&module_path) {
            if let Some((i, _)) = owner_hwnds.iter().enumerate().find(|(_, v)| **v == hwnd) {
                pid = get_window_pid(hwnds[i]);
                module_path = get_module_path(pid).unwrap_or_default();
            }
        }
        if is_valid_module_path(&module_path) {
            if !is_admin {
                if let Some(true) = is_process_elevated(pid) {
                    continue;
                }
            }
            result.entry(module_path).or_default().push((hwnd, title));
        }
    }
    debug!("list windows {result:?}");
    Ok(result)
}

fn is_valid_module_path(module_path: &str) -> bool {
    !module_path.is_empty() && module_path != "C:\\Windows\\System32\\ApplicationFrameHost.exe"
}

extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows: &mut Vec<HWND> = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
    windows.push(hwnd);
    BOOL(1)
}
