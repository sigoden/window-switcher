use crate::{log_info, virtual_desktop::VirtualDesktop};
use anyhow::{anyhow, Result};
use std::collections::{BTreeMap, BTreeSet};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, PWSTR};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_INFORMATION,
    PROCESS_VM_READ,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindowPlacement, GetWindowTextW, GetWindowThreadProcessId,
    IsWindowVisible, SetForegroundWindow, ShowWindow, SHOW_WINDOW_CMD, SW_RESTORE,
    SW_SHOWMINIMIZED, WINDOWPLACEMENT,
};

struct State {
    windows: BTreeMap<String, BTreeSet<isize>>,
    vitual_deskop: VirtualDesktop,
}

pub fn switch_next_window(virtual_deskop: &VirtualDesktop) -> Result<bool> {
    let mut state = State {
        windows: BTreeMap::default(),
        vitual_deskop: virtual_deskop.clone(),
    };

    unsafe { EnumWindows(Some(enum_window), LPARAM(&mut state as *mut _ as isize)).ok() }
        .map_err(|e| anyhow!("Fail to enum windows {}", e))?;

    let hwnd = match get_next_window(&state) {
        None => return Ok(false),
        Some(v) => v,
    };
    if get_window_placement(hwnd) == SW_SHOWMINIMIZED {
        unsafe { ShowWindow(hwnd, SW_RESTORE) }
            .ok()
            .map_err(|e| anyhow!("Fail to show window, {}", e))?;
    }
    unsafe { SetForegroundWindow(hwnd) }
        .ok()
        .map_err(|e| anyhow!("Fail to set window to foreground, {}", e))?;
    Ok(true)
}

extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let state: &mut State = unsafe { &mut *(lparam.0 as *mut State) };

    let ok: BOOL = true.into();

    if !state
        .vitual_deskop
        .is_window_on_current_virtual_desktop(hwnd)
        .unwrap_or_default()
    {
        return ok;
    }

    if !is_window_visible(hwnd) {
        return ok;
    }
    let title = get_window_title(hwnd);
    if title.is_empty() {
        return ok;
    }
    let pid = get_window_pid(hwnd);
    let module_path = get_module_path(pid);
    if module_path.is_empty() {
        return ok;
    }
    if &title == "Program Manager" && module_path.contains("explorer.exe") {
        return ok;
    }
    log_info!("{:?} {} {} {}", hwnd, pid, &title, &module_path);
    state.windows.entry(module_path).or_default().insert(hwnd.0);
    true.into()
}

fn get_next_window(state: &State) -> Option<HWND> {
    let (hwnd, pid) = unsafe {
        let hwnd = GetForegroundWindow();
        let pid = get_window_pid(hwnd);
        (hwnd, pid)
    };
    let module_path = get_module_path(pid);
    if module_path.is_empty() {
        return None;
    }
    match state.windows.get(&module_path) {
        None => None,
        Some(windows) => {
            log_info!("Switch windows {:?}", windows);
            let len = windows.len();
            if len == 1 {
                return None;
            }
            let values: Vec<isize> = windows.iter().cloned().collect();
            let index = windows.iter().position(|v| *v == hwnd.0)?;
            let new_index = (index + 1) % len;
            let new_hwnd = HWND(values[new_index]);
            log_info!("switch to {} {:?}", new_index, new_hwnd);
            Some(new_hwnd)
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

fn get_window_placement(hwnd: HWND) -> SHOW_WINDOW_CMD {
    let mut placement = WINDOWPLACEMENT::default();
    unsafe { GetWindowPlacement(hwnd, &mut placement) };
    placement.showCmd
}

fn get_window_title(hwnd: HWND) -> String {
    let buf = [0u16; 512];
    let len = buf.len();
    let len = unsafe { GetWindowTextW(hwnd, PWSTR(buf.as_ptr()), len as i32) };
    if len == 0 {
        return String::default();
    }
    String::from_utf16_lossy(&buf[..len as usize])
}

fn get_module_path(pid: u32) -> String {
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
