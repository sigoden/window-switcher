use anyhow::{anyhow, Result};
use windows::core::Error;
use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TOPMOST};
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::{SetLastError, ERROR_SUCCESS, HANDLE, HWND},
        Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWM_CLOAKED_SHELL},
        System::{
            LibraryLoader::GetModuleFileNameW,
            Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
            },
        },
        UI::{
            Controls::STATE_SYSTEM_INVISIBLE,
            Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, MOD_NOREPEAT},
            WindowsAndMessaging::{
                GetAncestor, GetForegroundWindow, GetLastActivePopup, GetTitleBarInfo,
                GetWindowThreadProcessId, IsIconic, IsWindowVisible, IsZoomed, SetForegroundWindow,
                ShowWindowAsync, GA_ROOTOWNER, GWL_USERDATA, SW_RESTORE, SW_SHOWMAXIMIZED,
                SW_SHOWNORMAL, TITLEBARINFO,
            },
        },
    },
};

use std::path::PathBuf;
use std::{ffi::c_void, mem::size_of};

use crate::HotKeyConfig;
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

pub fn get_module_path(pid: u32) -> String {
    let handle =
        match unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, None, pid) } {
            Ok(v) => v,
            Err(_) => {
                return String::default();
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
    String::from_utf16_lossy(&name)
}

pub fn get_window_exe(hwnd: HWND) -> String {
    let pid = get_window_pid(hwnd);
    if pid == 0 {
        return String::new();
    }
    let module_path = get_module_path(pid);
    get_basename(&module_path)
}

pub fn get_basename(path: &str) -> String {
    path.split('\\').last().unwrap_or_default().to_lowercase()
}

pub fn is_iconic(hwnd: HWND) -> bool {
    unsafe { IsIconic(hwnd) }.as_bool()
}

pub fn is_zoombed(hwnd: HWND) -> bool {
    unsafe { IsZoomed(hwnd) }.as_bool()
}

pub fn is_window_visible(hwnd: HWND) -> bool {
    let ret = unsafe { IsWindowVisible(hwnd) };
    ret.as_bool()
}

pub fn is_window_fixed_top(hwnd: HWND) -> bool {
    let ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    ex_style & WS_EX_TOPMOST.0 != 0
}

pub fn is_window_cloaked(hwnd: HWND) -> bool {
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

        if is_window_visible(hwnd_try) {
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

pub fn get_foreground_window() -> HWND {
    unsafe { GetForegroundWindow() }
}

pub fn switch_to(hwnd: HWND) -> Result<()> {
    if hwnd == get_foreground_window() {
        return Ok(());
    }
    if let Err(err) = unsafe { SetForegroundWindow(hwnd) }.ok() {
        error!("Failed to set foreground {hwnd:?}, {err}");
    }

    unsafe {
        if is_iconic(hwnd) {
            ShowWindowAsync(hwnd, SW_RESTORE);
        } else if is_zoombed(hwnd) {
            ShowWindowAsync(hwnd, SW_SHOWMAXIMIZED);
        } else {
            ShowWindowAsync(hwnd, SW_SHOWNORMAL);
        }
    };
    Ok(())
}

pub fn register_hotkey(hwnd: HWND, id: u32, hotkey: &HotKeyConfig) -> Result<()> {
    unsafe {
        RegisterHotKey(
            hwnd,
            id as i32,
            hotkey.hotkey_modifier() | MOD_NOREPEAT,
            hotkey.code as u32,
        )
    }
    .ok()
    .map_err(|e| anyhow!("Fail to register hotkey {id}, {e}"))
}

pub fn unregister_hotkey(hwnd: HWND, id: u32) -> Result<()> {
    unsafe { UnregisterHotKey(hwnd, id as i32) }
        .ok()
        .map_err(|e| anyhow!("Fail to unregister hotkey {id}, {e}"))
}

#[cfg(target_arch = "x86")]
pub fn get_window_ptr(hwnd: HWND) -> i32 {
    unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowLongW(hwnd, GWL_USERDATA) }
}
#[cfg(target_arch = "x86_64")]
pub fn get_window_ptr(hwnd: HWND) -> isize {
    unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd, GWL_USERDATA) }
}

#[cfg(target_arch = "x86")]
pub fn set_window_ptr(hwnd: HWND, ptr: i32) -> i32 {
    unsafe { windows::Win32::UI::WindowsAndMessaging::SetWindowLongW(hwnd, GWL_USERDATA, ptr) }
}

#[cfg(target_arch = "x86_64")]
pub fn set_window_ptr(hwnd: HWND, ptr: isize) -> isize {
    unsafe { windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(hwnd, GWL_USERDATA, ptr) }
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
