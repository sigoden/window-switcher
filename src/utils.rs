use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use windows::core::{Error, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    CloseHandle, SetLastError, BOOL, ERROR_ALREADY_EXISTS, ERROR_SUCCESS, HANDLE, HWND, LPARAM,
    TRUE, WPARAM,
};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWM_CLOAKED_SHELL};
use windows::Win32::System::Console::{AllocConsole, FreeConsole, GetConsoleWindow};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Threading::{
    CreateMutexW, OpenProcess, QueryFullProcessImageNameW, ReleaseMutex, PROCESS_NAME_WIN32,
    PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
};
use windows::Win32::UI::Controls::STATE_SYSTEM_INVISIBLE;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconFromResourceEx, EnumChildWindows, EnumWindows, GetAncestor, GetForegroundWindow,
    GetLastActivePopup, GetTitleBarInfo, GetWindowLongPtrW, GetWindowPlacement,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, LoadIconW, SendMessageW,
    SetForegroundWindow, SetWindowPos, ShowWindow, GA_ROOTOWNER, GCL_HICON, GWL_EXSTYLE,
    GWL_USERDATA, HICON, ICON_BIG, IDI_APPLICATION, LR_DEFAULTCOLOR, SWP_NOZORDER, SW_RESTORE,
    TITLEBARINFO, WINDOWPLACEMENT, WM_GETICON, WS_EX_TOPMOST,
};

use std::fs::{read_dir, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::{ffi::c_void, mem::size_of};
use xml::reader::XmlEvent;
use xml::EventReader;

pub const BUFFER_SIZE: usize = 1024;

pub fn get_exe_folder() -> Result<PathBuf> {
    let path =
        std::env::current_exe().map_err(|err| anyhow!("Failed to get binary path, {err}"))?;
    path.parent()
        .ok_or_else(|| anyhow!("Failed to get binary folder"))
        .map(|v| v.to_path_buf())
}

pub fn get_exe_path() -> Vec<u16> {
    let mut path = vec![0u16; BUFFER_SIZE];
    let size = unsafe { GetModuleFileNameW(None, &mut path) } as usize;
    path[..size].to_vec()
}

pub fn get_window_pid(hwnd: HWND) -> u32 {
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid as *mut u32)) };
    pid
}

pub fn get_module_path(hwnd: HWND, pid: u32) -> String {
    let handle =
        match unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, None, pid) } {
            Ok(v) => v,
            Err(_) => {
                return String::new();
            }
        };
    let mut len: u32 = 1024;
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
        return String::default();
    }
    unsafe { name.set_len(len as usize) };
    let mut module_path = String::from_utf16_lossy(&name);
    if module_path.ends_with("ApplicationFrameHost.exe") {
        module_path = get_modern_app_path(hwnd).unwrap_or_default();
    }
    module_path
}

pub fn get_window_exe(hwnd: HWND) -> String {
    let pid = get_window_pid(hwnd);
    if pid == 0 {
        return String::new();
    }
    let module_path = get_module_path(hwnd, pid);
    get_basename(&module_path)
}

pub fn get_basename(path: &str) -> String {
    path.split('\\').last().unwrap_or_default().to_lowercase()
}

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
    cloaked != 0 && DWM_CLOAKED_SHELL != 0
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
    let mut placement = WINDOWPLACEMENT::default();
    unsafe { GetWindowPlacement(hwnd, &mut placement) };
    let rect = placement.rcNormalPosition;
    (rect.right - rect.left) * (rect.bottom - rect.top) < 5000
}

pub fn get_foreground_window() -> HWND {
    unsafe { GetForegroundWindow() }
}

pub fn set_foregound_window(hwnd: HWND) -> Result<()> {
    unsafe {
        if is_iconic_window(hwnd) {
            ShowWindow(hwnd, SW_RESTORE);
        }
        if hwnd == get_foreground_window() {
            return Ok(());
        }
        if SetForegroundWindow(hwnd).ok().is_err() {
            AllocConsole();
            let hwnd_console = GetConsoleWindow();
            SetWindowPos(hwnd_console, None, 0, 0, 0, 0, SWP_NOZORDER);
            FreeConsole();
            SetForegroundWindow(hwnd);
        }
    };
    Ok(())
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

pub fn create_hicon_from_resource(data: &[u8]) -> Option<HICON> {
    unsafe { CreateIconFromResourceEx(data, TRUE, 0x30000, 100, 100, LR_DEFAULTCOLOR) }
        .ok()
        .or_else(|| unsafe { LoadIconW(None, IDI_APPLICATION) }.ok())
}

pub fn list_windows(is_switch_apps: bool) -> Result<IndexMap<String, Vec<HWND>>> {
    let mut data = EnumWindowsData {
        is_switch_apps,
        windows: Default::default(),
    };
    unsafe { EnumWindows(Some(enum_window), LPARAM(&mut data as *mut _ as isize)).ok() }
        .map_err(|e| anyhow!("Fail to get windows {}", e))?;
    debug!("list windows {:?} {is_switch_apps}", data.windows);
    Ok(data.windows)
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

#[allow(unused)]
#[inline]
/// Use to wrap fallible Win32 functions.
/// First calls SetLastError(0).
/// And then after checks GetLastError().
/// Useful when the return value doesn't reliably indicate failure.
pub fn check_error<F, R>(mut f: F) -> windows::core::Result<R>
where
    F: FnMut() -> R,
{
    unsafe {
        SetLastError(ERROR_SUCCESS);
        let result = f();
        let error = Error::from_win32();
        if error == Error::OK {
            Ok(result)
        } else {
            Err(error)
        }
    }
}

pub trait CheckError: Sized {
    fn check_error(self) -> windows::core::Result<Self>;
}

impl CheckError for HANDLE {
    fn check_error(self) -> windows::core::Result<Self> {
        if self.is_invalid() {
            Err(Error::from_win32())
        } else {
            Ok(self)
        }
    }
}

impl CheckError for HWND {
    fn check_error(self) -> windows::core::Result<Self> {
        // If the function fails, the return value is NULL.
        if self.0 == 0 {
            Err(Error::from_win32())
        } else {
            Ok(self)
        }
    }
}

impl CheckError for u16 {
    fn check_error(self) -> windows::core::Result<Self> {
        // If the function fails, the return value is zero
        if self == 0 {
            Err(Error::from_win32())
        } else {
            Ok(self)
        }
    }
}

pub fn to_wstring(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect::<Vec<u16>>()
}

/// A struct representing one running instance.
pub struct SingleInstance {
    handle: Option<HANDLE>,
}

unsafe impl Send for SingleInstance {}
unsafe impl Sync for SingleInstance {}

impl SingleInstance {
    /// Returns a new SingleInstance object.
    pub fn create(name: &str) -> Result<Self> {
        let name = to_wstring(name);
        let handle = unsafe { CreateMutexW(None, BOOL(1), PCWSTR(name.as_ptr())) }
            .map_err(|err| anyhow!("Fail to setup single instance, {err}"))?;
        let handle =
            if windows::core::Error::from_win32().code() == ERROR_ALREADY_EXISTS.to_hresult() {
                None
            } else {
                Some(handle)
            };
        Ok(SingleInstance { handle })
    }

    /// Returns whether this instance is single.
    pub fn is_single(&self) -> bool {
        self.handle.is_some()
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            unsafe {
                ReleaseMutex(handle);
                CloseHandle(handle);
            }
        }
    }
}

#[derive(Debug)]
struct EnumWindowsData {
    is_switch_apps: bool,
    windows: IndexMap<String, Vec<HWND>>,
}

extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let state: &mut EnumWindowsData = unsafe { &mut *(lparam.0 as *mut _) };
    if !is_visible_window(hwnd)
        || is_special_window(hwnd)
        || is_small_window(hwnd)
        || is_cloaked_window(hwnd)
        || is_popup_window(hwnd)
    {
        return BOOL(1);
    }
    if state.is_switch_apps && (is_iconic_window(hwnd) || is_topmost_window(hwnd)) {
        return BOOL(1);
    }
    let pid = get_window_pid(hwnd);
    let module_path = get_module_path(hwnd, pid);
    state.windows.entry(module_path).or_default().push(hwnd);
    BOOL(1)
}

fn get_modern_app_path(hwnd: HWND) -> Option<String> {
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
            return Some(get_module_path(child_hwnd, child_pid));
        }
    }
    None
}

extern "system" fn enum_child_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows: &mut Vec<HWND> = unsafe { &mut *(lparam.0 as *mut _) };
    windows.push(hwnd);
    BOOL(1)
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
