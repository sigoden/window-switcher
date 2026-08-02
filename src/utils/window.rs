use crate::utils::is_process_elevated;

use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use std::{ffi::c_void, mem::size_of, path::PathBuf};
use windows::core::{BOOL, PCWSTR, PWSTR};
use windows::Win32::{
    Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, HWND, LPARAM, MAX_PATH, POINT, RECT},
    Graphics::{
        Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWM_CLOAKED_SHELL},
        Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST},
    },
    Storage::{
        EnhancedStorage::PKEY_AppUserModel_ID,
        Packaging::Appx::{GetPackagePathByFullName, GetPackagesByPackageFamily},
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
        Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow},
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

fn is_chrome_browser(module_path: &str) -> bool {
    let lower = module_path.to_lowercase();
    lower.ends_with("chrome.exe")
}

fn is_edge_browser(module_path: &str) -> bool {
    let lower = module_path.to_lowercase();
    lower.ends_with("msedge.exe")
}

fn get_aumid(hwnd: HWND) -> Option<String> {
    let store: IPropertyStore = unsafe { SHGetPropertyStoreForWindow(hwnd).ok()? };
    let propvar = unsafe { store.GetValue(&PKEY_AppUserModel_ID).ok()? };
    Some(propvar.to_string())
}

fn get_edge_aumid_info(hwnd: HWND) -> (Option<String>, Option<String>) {
    let aumid = match get_aumid(hwnd) {
        Some(v) => v,
        None => return (None, None),
    };

    if let Some(pkg) = aumid.strip_suffix("!App") {
        if !pkg.is_empty() {
            return (None, Some(pkg.to_string()));
        }
        return (None, None);
    }

    if aumid == "MSEdge" || aumid.is_empty() {
        return (None, None);
    }
    if let Some(profile) = aumid.strip_prefix("MSEdge.UserData.") {
        if !profile.is_empty() {
            return (Some(profile.to_string()), None);
        }
    }
    (None, None)
}

fn get_chrome_aumid_info(hwnd: HWND) -> (Option<String>, Option<String>) {
    let aumid = match get_aumid(hwnd) {
        Some(v) => v,
        None => return (None, None),
    };

    let crx_prefix = "_crx_";
    if let Some(idx) = aumid.find(crx_prefix) {
        let after_crx = &aumid[idx + crx_prefix.len()..];
        let (app_id, profile) = if let Some(dot_idx) = after_crx.find(".UserData.") {
            (
                &after_crx[..dot_idx],
                &after_crx[dot_idx + ".UserData.".len()..],
            )
        } else {
            (after_crx, "Default")
        };
        if !app_id.is_empty() && !profile.is_empty() {
            let profile = if profile == "Default" {
                None
            } else {
                Some(profile.to_string())
            };
            return (profile, Some(app_id.to_string()));
        }
        return (None, None);
    }

    if aumid == "Chrome" || aumid.is_empty() {
        return (None, None);
    }
    if let Some(profile) = aumid.strip_prefix("Chrome.UserData.") {
        if !profile.is_empty() {
            return (Some(profile.to_string()), None);
        }
    }
    (None, None)
}

fn is_app_id_match(full: &str, truncated: &str) -> bool {
    if full.eq_ignore_ascii_case(truncated) {
        return true;
    }
    let mut chars = full.chars();
    for tc in truncated.chars() {
        loop {
            match chars.next() {
                Some(fc) if fc == tc => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

pub(crate) fn pwa_map_profile_dir(aumid_profile: &str) -> String {
    if let Some(num) = aumid_profile.strip_prefix("Profile") {
        if num.chars().all(|c| c.is_ascii_digit()) {
            return format!("Profile {}", num);
        }
    }
    aumid_profile.to_string()
}

pub(crate) fn pwa_find_lnk_path(
    user_data_dir: &str,
    profile: &str,
    app_id: &str,
) -> Option<PathBuf> {
    let profile_dir = pwa_map_profile_dir(profile);
    let web_apps_dir = PathBuf::from(user_data_dir)
        .join(&profile_dir)
        .join("Web Applications");
    if !web_apps_dir.is_dir() {
        return None;
    }
    for entry in std::fs::read_dir(&web_apps_dir).ok()?.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let dir_name = entry.file_name().to_string_lossy().to_string();
        let dir_app_id = dir_name.strip_prefix("_crx_").unwrap_or(&dir_name);
        if is_app_id_match(dir_app_id, app_id) {
            for lnk_entry in std::fs::read_dir(entry.path()).ok()?.flatten() {
                let path = lnk_entry.path();
                if path.extension().map(|e| e.to_string_lossy().to_lowercase())
                    == Some("lnk".into())
                {
                    return Some(path);
                }
            }
            return None;
        }
    }
    None
}

pub(crate) fn find_appx_pkg_dir(package_family_name: &str) -> Option<String> {
    unsafe {
        let pfn: Vec<u16> = package_family_name.encode_utf16().chain(Some(0)).collect();

        let mut count = 0u32;
        let mut buffer_len = 0u32;

        let rc = GetPackagesByPackageFamily(
            PCWSTR(pfn.as_ptr()),
            &mut count,
            None,
            &mut buffer_len,
            None,
        );

        if rc.0 != ERROR_INSUFFICIENT_BUFFER.0 {
            return None;
        }

        let mut names = vec![PWSTR::null(); count as usize];
        let mut buffer = vec![0u16; buffer_len as usize];

        if GetPackagesByPackageFamily(
            PCWSTR(pfn.as_ptr()),
            &mut count,
            Some(names.as_mut_ptr()),
            &mut buffer_len,
            Some(PWSTR(buffer.as_mut_ptr())),
        )
        .0 != ERROR_SUCCESS.0
        {
            return None;
        }

        for name in names {
            let full_name = name.to_string().ok()?;
            let full_w: Vec<u16> = full_name.encode_utf16().chain(Some(0)).collect();

            let mut path_len = 0u32;

            let _ = GetPackagePathByFullName(PCWSTR(full_w.as_ptr()), &mut path_len, None);

            let mut path_buf = vec![0u16; path_len as usize];

            if GetPackagePathByFullName(
                PCWSTR(full_w.as_ptr()),
                &mut path_len,
                Some(PWSTR(path_buf.as_mut_ptr())),
            )
            .0 != ERROR_SUCCESS.0
            {
                continue;
            }

            let path = String::from_utf16_lossy(&path_buf[..(path_len as usize - 1)]);
            return Some(path);
        }
    }
    None
}

pub(crate) fn get_default_user_data_dir(module_path: &str) -> Option<String> {
    let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
    let browser_path = if module_path.to_lowercase().contains("chrome.exe") {
        r"Google\Chrome\User Data"
    } else if module_path.to_lowercase().contains("msedge.exe") {
        r"Microsoft\Edge\User Data"
    } else {
        return None;
    };
    Some(
        PathBuf::from(local_app_data)
            .join(browser_path)
            .to_string_lossy()
            .to_string(),
    )
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
    merge_browser_profiles: bool,
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
            let key = if merge_browser_profiles {
                module_path.clone()
            } else if is_chrome_browser(&module_path) {
                match get_chrome_aumid_info(hwnd) {
                    (Some(profile), Some(app_id)) => {
                        format!("{}::{}::{}", module_path, profile, app_id)
                    }
                    (Some(profile), None) => format!("{}::{}", module_path, profile),
                    (None, Some(app_id)) => format!("{}::Default::{}", module_path, app_id),
                    (None, None) => module_path.clone(),
                }
            } else if is_edge_browser(&module_path) {
                match get_edge_aumid_info(hwnd) {
                    (None, Some(pkg)) => format!("{}::appx::{}", module_path, pkg),
                    (Some(profile), None) => format!("{}::{}", module_path, profile),
                    _ => module_path.clone(),
                }
            } else {
                module_path.clone()
            };
            result.entry(key).or_default().push((hwnd, title));
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
