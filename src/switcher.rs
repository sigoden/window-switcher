use crate::{log_error, log_info};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::ops::Deref;
use std::time::{Duration, Instant};
use windows::core::GUID;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, PWSTR};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_INFORMATION,
    PROCESS_VM_READ,
};
use windows::Win32::UI::Shell::IVirtualDesktopManager;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindowPlacement, GetWindowTextW, GetWindowThreadProcessId,
    IsWindowVisible, SetForegroundWindow, ShowWindow, SHOW_WINDOW_CMD, SW_RESTORE,
    SW_SHOWMINIMIZED, WINDOWPLACEMENT,
};

const LONG_INTERVAL: Duration = Duration::from_secs(3);
#[allow(non_upper_case_globals)]
const CLSID_VirtualDesktopManager: GUID = GUID::from_u128(0xaa509086_5ca9_4c25_8f95_589d3c07b48a);

pub struct Switcher {
    windows: HashMap<String, Vec<isize>>,
    virtual_desktop: Option<VirtualDesktop>,
    last_switch_info: Option<SwitchInfo>,
}

impl Switcher {
    pub fn new(virtual_desktop: Option<VirtualDesktop>) -> Self {
        Self {
            windows: HashMap::new(),
            virtual_desktop,
            last_switch_info: None,
        }
    }

    pub fn switch_window(&mut self) -> Result<bool> {
        self.enum_windows()?;

        let current_window = unsafe { GetForegroundWindow() };
        let pid = get_window_pid(current_window);
        let module_path = get_module_path(pid);
        if module_path.is_empty() {
            return Ok(false);
        }
        let (index, hwnd) = match self.windows.get(&module_path) {
            None => return Ok(false),
            Some(windows) => {
                log_info!("available switch windows {:?}", windows);
                let len = windows.len();
                if len == 1 {
                    return Ok(false);
                }
                let mut index = 1;
                if let Some(info) = self.last_switch_info.as_ref() {
                    log_info!("last switch info {:?}", info);
                    if info.path == module_path && info.at.elapsed() < LONG_INTERVAL {
                        index = (info.index + 1).min(windows.len() - 1);
                    }
                }
                (index, HWND(windows[index]))
            }
        };

        log_info!("switch to {:?} at {}", hwnd, index);
        self.switch_to(hwnd)?;

        self.windows.clear();

        self.last_switch_info = Some(SwitchInfo::new(&module_path, index));

        Ok(true)
    }

    fn enum_windows(&mut self) -> Result<()> {
        unsafe { EnumWindows(Some(enum_window), LPARAM(self as *mut _ as isize)).ok() }
            .map_err(|e| anyhow!("Fail to enum windows {}", e))
    }

    fn switch_to(&self, hwnd: HWND) -> Result<()> {
        if get_window_placement(hwnd) == SW_SHOWMINIMIZED {
            unsafe { ShowWindow(hwnd, SW_RESTORE) }
                .ok()
                .map_err(|e| anyhow!("Fail to show window, {}", e))?;
        }
        unsafe { SetForegroundWindow(hwnd) }
            .ok()
            .map_err(|e| anyhow!("Fail to set window to foreground, {}", e))
    }

    fn is_window_on_desktop(&self, hwnd: HWND) -> bool {
        if let Some(virtual_desktop) = self.virtual_desktop.as_ref() {
            match virtual_desktop.is_window_on_current_virtual_desktop(hwnd) {
                Ok(on) => {
                    if !on {
                        return false;
                    }
                }
                Err(err) => {
                    log_error!(err.to_string());
                }
            }
        }
        true
    }
}

#[derive(Debug)]
struct SwitchInfo {
    at: Instant,
    path: String,
    index: usize,
}

impl SwitchInfo {
    fn new(path: &str, index: usize) -> Self {
        Self {
            at: Instant::now(),
            path: path.to_string(),
            index,
        }
    }
}

#[derive(Clone)]
pub struct VirtualDesktop {
    inner: IVirtualDesktopManager,
}

impl VirtualDesktop {
    pub fn create() -> Result<Self> {
        let inner = unsafe {
            CoCreateInstance(&CLSID_VirtualDesktopManager, None, CLSCTX_ALL)
                .map_err(|e| anyhow!("Fail to access virtual desktop com, {}", e))?
        };
        Ok(Self { inner })
    }
    pub fn is_window_on_current_virtual_desktop(&self, hwnd: HWND) -> Result<bool> {
        let ret = unsafe { self.inner.IsWindowOnCurrentVirtualDesktop(hwnd) }
            .map_err(|e| anyhow!("Fail to check current desktop, {}", e))?;
        Ok(ret.as_bool())
    }
}

impl Deref for VirtualDesktop {
    type Target = IVirtualDesktopManager;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let switcher: &mut Switcher = unsafe { &mut *(lparam.0 as *mut Switcher) };

    let ok: BOOL = true.into();

    if !switcher.is_window_on_desktop(hwnd) {
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
    switcher
        .windows
        .entry(module_path)
        .or_default()
        .push(hwnd.0);

    true.into()
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
