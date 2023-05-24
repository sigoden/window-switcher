use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use windows::core::PWSTR;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, MAX_PATH, RECT, TRUE, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::System::Console::{AllocConsole, FreeConsole, GetConsoleWindow};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::ProcessStatus::EnumProcesses;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_INFORMATION,
    PROCESS_VM_READ,
};
use windows::Win32::UI::Controls::STATE_SYSTEM_INVISIBLE;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconFromResourceEx, EnumChildWindows, EnumWindows, GetAncestor, GetForegroundWindow,
    GetLastActivePopup, GetTitleBarInfo, GetWindow, GetWindowLongPtrW, GetWindowPlacement,
    GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible, LoadIconW, SendMessageW,
    SetForegroundWindow, SetWindowPos, ShowWindow, GA_ROOTOWNER, GCL_HICON, GWL_EXSTYLE,
    GWL_USERDATA, GW_OWNER, HICON, ICON_BIG, IDI_APPLICATION, LR_DEFAULTCOLOR, SWP_NOZORDER,
    SW_RESTORE, TITLEBARINFO, WINDOWPLACEMENT, WM_GETICON, WS_EX_TOPMOST,
};

use std::fs::{read_dir, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::{ffi::c_void, mem::size_of};
use xml::reader::XmlEvent;
use xml::EventReader;

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

pub fn is_popup_window(hwnd: HWND) -> bool {
    let mut wnd_walk = HWND::default();

    // Start at the root owner
    let mut hwnd_try = unsafe { GetAncestor(hwnd, GA_ROOTOWNER) };

    // See if we are the last active visible popup
    while hwnd_try != wnd_walk {
        wnd_walk = hwnd_try;
        hwnd_try = unsafe { GetLastActivePopup(wnd_walk) };

        if is_visible_window(hwnd_try) {
            break;
        }
    }
    wnd_walk != hwnd
}

pub fn is_special_window(hwnd: HWND) -> bool {
    // like task tray programs and "Program Manager"
    let mut ti: TITLEBARINFO = TITLEBARINFO {
        cbSize: size_of::<TITLEBARINFO>() as u32,
        ..Default::default()
    };

    unsafe {
        GetTitleBarInfo(hwnd, &mut ti);
    }

    ti.rgstate[0] & STATE_SYSTEM_INVISIBLE.0 != 0
}

pub fn is_small_window(hwnd: HWND) -> bool {
    let rect = get_window_rect(hwnd);
    (rect.right - rect.left) < 150 || (rect.bottom - rect.top) < 50
}

pub fn is_show_window(hwnd: HWND) -> bool {
    if !is_visible_window(hwnd) {
        return false;
    }
    if is_small_window(hwnd) {
        return false;
    }
    if is_topmost_window(hwnd) {
        return false;
    }
    let title = get_window_title(hwnd);
    if title.is_empty() {
        return false;
    }
    if is_special_window(hwnd)
        && [
            "Program Manager",
            "Settings",
            "Microsoft Text Input Application",
        ]
        .contains(&title.as_str())
    {
        return false;
    }
    true
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

pub fn get_module_path(hwnd: HWND, pid: u32) -> Option<String> {
    let module_path = get_module_path_impl(pid)?;
    if module_path.ends_with("ApplicationFrameHost.exe") {
        get_modern_app_path(hwnd)
    } else {
        Some(module_path)
    }
}

fn get_module_path_impl(pid: u32) -> Option<String> {
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
    if !ret.as_bool() || len == 0 {
        return None;
    }
    unsafe { name.set_len(len as usize) };
    let module_path = String::from_utf16_lossy(&name);
    if module_path.is_empty() {
        return None;
    }
    Some(module_path)
}

pub fn get_modern_app_path(hwnd: HWND) -> Option<String> {
    let pid = get_window_pid(hwnd);
    let mut child_windows: Vec<HWND> = vec![];
    unsafe {
        EnumChildWindows(
            hwnd,
            Some(enum_child_window),
            LPARAM(&mut child_windows as *mut _ as isize),
        )
        .ok()
        .ok()?
    };
    for child_hwnd in child_windows {
        let child_pid = get_window_pid(child_hwnd);
        if child_pid != pid {
            return get_module_path_impl(child_pid);
        }
    }
    None
}

extern "system" fn enum_child_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows: &mut Vec<HWND> = unsafe { &mut *(lparam.0 as *mut _) };
    windows.push(hwnd);
    BOOL(1)
}

pub fn get_window_exe(hwnd: HWND) -> Option<String> {
    let pid = get_window_pid(hwnd);
    if pid == 0 {
        return None;
    }
    let module_path = get_module_path(hwnd, pid)?;
    module_path.split('\\').map(|v| v.to_string()).last()
}

pub fn set_foregound_window(hwnd: HWND, module_path: &str) {
    unsafe {
        if is_cloaked_window(hwnd) && module_path.starts_with("C:\\Program Files\\WindowsApps") {
            show_uwp_window(module_path);
            return;
        }
        if is_iconic_window(hwnd) {
            ShowWindow(hwnd, SW_RESTORE);
        }
        if hwnd == get_foreground_window() {
            return;
        }
        if SetForegroundWindow(hwnd).ok().is_err() {
            AllocConsole();
            let hwnd_console = GetConsoleWindow();
            SetWindowPos(hwnd_console, None, 0, 0, 0, 0, SWP_NOZORDER);
            FreeConsole();
            SetForegroundWindow(hwnd);
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

pub fn get_window_rect(hwnd: HWND) -> RECT {
    let mut placement = WINDOWPLACEMENT::default();
    unsafe { GetWindowPlacement(hwnd, &mut placement) };
    placement.rcNormalPosition
}

pub fn get_module_icon(hwnd: HWND) -> Option<HICON> {
    let ret = unsafe { SendMessageW(hwnd, WM_GETICON, WPARAM(ICON_BIG as _), None) }.0;
    if ret != 0 {
        return Some(HICON(ret));
    }

    let ret = get_class_icon(hwnd);
    if ret != 0 {
        return Some(HICON(ret as _));
    }

    unsafe { LoadIconW(None, IDI_APPLICATION) }.ok()
}

#[cfg(target_arch = "x86")]
pub fn get_window_user_data(hwnd: HWND) -> i32 {
    unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowLongW(hwnd, GWL_USERDATA) }
}
#[cfg(target_arch = "x86_64")]
pub fn get_window_user_data(hwnd: HWND) -> isize {
    unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd, GWL_USERDATA) }
}

#[cfg(target_arch = "x86")]
pub fn set_window_user_data(hwnd: HWND, ptr: i32) -> i32 {
    unsafe { windows::Win32::UI::WindowsAndMessaging::SetWindowLongW(hwnd, GWL_USERDATA, ptr) }
}

#[cfg(target_arch = "x86_64")]
pub fn set_window_user_data(hwnd: HWND, ptr: isize) -> isize {
    unsafe { windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(hwnd, GWL_USERDATA, ptr) }
}

#[cfg(target_arch = "x86")]
pub fn get_class_icon(hwnd: HWND) -> u32 {
    unsafe { windows::Win32::UI::WindowsAndMessaging::GetClassLongW(hwnd, GCL_HICON) }
}
#[cfg(target_arch = "x86_64")]
pub fn get_class_icon(hwnd: HWND) -> usize {
    unsafe { windows::Win32::UI::WindowsAndMessaging::GetClassLongPtrW(hwnd, GCL_HICON) }
}

pub fn list_windows(_is_switch_apps: bool) -> Result<IndexMap<String, Vec<HWND>>> {
    let mut data = EnumWindowsData {
        _is_switch_apps,
        windows: Default::default(),
    };
    unsafe { EnumWindows(Some(enum_window), LPARAM(&mut data as *mut _ as isize)).ok() }
        .map_err(|e| anyhow!("Fail to get windows {}", e))?;
    debug!("list windows {:?} {_is_switch_apps}", data.windows);
    Ok(data.windows)
}

#[derive(Debug)]
struct EnumWindowsData {
    _is_switch_apps: bool,
    windows: IndexMap<String, Vec<HWND>>,
}

extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let state: &mut EnumWindowsData = unsafe { &mut *(lparam.0 as *mut _) };
    if !is_show_window(hwnd) {
        return BOOL(1);
    }
    let pid = get_window_pid(hwnd);
    let module_path = match get_module_path(hwnd, pid) {
        Some(v) => v,
        None => return BOOL(1),
    };
    state.windows.entry(module_path).or_default().push(hwnd);
    BOOL(1)
}

pub fn get_uwp_icon_data(module_path: &str) -> Option<Vec<u8>> {
    let module_path = PathBuf::from(module_path);
    let module_dir = module_path.parent()?;
    let manifest_path = module_dir.join("AppxManifest.xml");
    let metadata_logo_path = get_appx_logo_path(&manifest_path)?;
    let metadata_logo_path = module_dir.join(metadata_logo_path);
    let logo_dir = metadata_logo_path.parent()?;
    let mut logo_path = None;
    let log_basename = metadata_logo_path
        .file_stem()?
        .to_string_lossy()
        .to_string();
    for entry in read_dir(logo_dir).ok()? {
        let entry = entry.ok()?;
        let entry_file_name = entry.file_name().to_string_lossy().to_string();
        if entry_file_name.starts_with(&log_basename) {
            logo_path = Some(logo_dir.join(entry_file_name).to_string_lossy().to_string());
            break;
        }
    }
    let logo_path = logo_path?;
    let mut logo_file = File::open(logo_path).ok()?;
    let mut buffer = vec![];
    logo_file.read_to_end(&mut buffer).ok()?;
    Some(buffer)
}

fn get_appx_logo_path(manifest_path: &Path) -> Option<String> {
    let manifest_file = File::open(manifest_path).ok()?;
    let manifest_file = BufReader::new(manifest_file); // Buffering is important for performance
    let reader = EventReader::new(manifest_file);
    let mut logo_path = None;
    let mut xpaths = vec![];
    let mut depth = 0;
    for e in reader {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                if xpaths.len() == depth {
                    xpaths.push(name.local_name.clone())
                }
                depth += 1;
            }
            Ok(XmlEvent::EndElement { .. }) => {
                if xpaths.len() == depth {
                    xpaths.pop();
                }
                depth -= 1;
            }
            Ok(XmlEvent::Characters(text)) => {
                if xpaths.join("/") == "Package/Properties/Logo" {
                    logo_path = Some(text);
                    break;
                }
            }
            Err(_) => {
                break;
            }
            _ => {}
        }
    }
    logo_path
}

pub fn create_hicon_from_resource(data: &[u8]) -> Option<HICON> {
    unsafe { CreateIconFromResourceEx(data, TRUE, 0x30000, 100, 100, LR_DEFAULTCOLOR) }
        .ok()
        .or_else(|| unsafe { LoadIconW(None, IDI_APPLICATION) }.ok())
}

fn show_uwp_window(module_path: &str) -> Option<HWND> {
    let pid = get_process_by_path(module_path)?;
    let mut data = EnumUwpWindowsData {
        pid,
        windows: vec![],
    };
    unsafe { EnumWindows(Some(enum_uwp_window), LPARAM(&mut data as *mut _ as isize)) };
    for hwnd in data.windows {
        let owner: HWND = unsafe { GetWindow(hwnd, GW_OWNER) };
        if owner != HWND(0) {
            unsafe { ShowWindow(owner, SW_RESTORE) };
        }
    }
    None
}

#[derive(Debug)]
struct EnumUwpWindowsData {
    pid: u32,
    windows: Vec<HWND>,
}

extern "system" fn enum_uwp_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let state: &mut EnumUwpWindowsData = unsafe { &mut *(lparam.0 as *mut _) };
    let pid = get_window_pid(hwnd);
    if pid == state.pid {
        state.windows.push(hwnd);
    }
    BOOL(1)
}

fn get_process_by_path(module_path: &str) -> Option<u32> {
    unsafe {
        let mut pids = [0; 4096];
        let mut pids_length = 0;
        EnumProcesses(
            pids.as_mut_ptr(),
            std::mem::size_of_val(&pids) as u32,
            &mut pids_length,
        )
        .ok()
        .unwrap();
        for &pid in &pids[..pids_length as usize / std::mem::size_of::<u32>()] {
            if let Some(path) = get_module_path_impl(pid) {
                if path == module_path {
                    return Some(pid);
                }
            }
        }
    }
    None
}
