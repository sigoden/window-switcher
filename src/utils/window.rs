use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use windows::core::PWSTR;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, MAX_PATH, TRUE, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::System::Console::{AllocConsole, FreeConsole, GetConsoleWindow};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_INFORMATION,
    PROCESS_VM_READ,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconFromResourceEx, EnumWindows, GetForegroundWindow, GetWindow, GetWindowLongPtrW,
    GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible, LoadIconW, SendMessageW,
    SetForegroundWindow, SetWindowPos, ShowWindow, GCL_HICON, GWL_EXSTYLE, GWL_USERDATA, GW_OWNER,
    HICON, ICON_BIG, IDI_APPLICATION, LR_DEFAULTCOLOR, SWP_NOZORDER, SW_RESTORE, WM_GETICON,
    WS_EX_TOPMOST,
};

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
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

pub fn get_window_exe(hwnd: HWND) -> Option<String> {
    let pid = get_window_pid(hwnd);
    if pid == 0 {
        return None;
    }
    let module_path = get_module_path(pid)?;
    module_path.split('\\').map(|v| v.to_string()).last()
}

pub fn set_foregound_window(hwnd: HWND) {
    unsafe {
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

pub fn get_owner_window(hwnd: HWND) -> HWND {
    unsafe { GetWindow(hwnd, GW_OWNER) }
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

pub fn list_windows() -> Result<IndexMap<String, Vec<(HWND, String)>>> {
    let mut result: IndexMap<String, Vec<(HWND, String)>> = IndexMap::new();
    let mut hwnds: Vec<HWND> = Default::default();
    unsafe { EnumWindows(Some(enum_window), LPARAM(&mut hwnds as *mut _ as isize)).ok() }
        .map_err(|e| anyhow!("Fail to get windows {}", e))?;
    let mut visiable_hwnds = vec![];
    let mut owner_hwnds = vec![];
    for hwnd in hwnds.iter().cloned() {
        if is_visible_window(hwnd) && !is_cloaked_window(hwnd) && !is_topmost_window(hwnd) {
            let title = get_window_title(hwnd);
            if !title.is_empty() && title != "Program Manager" {
                visiable_hwnds.push((hwnd, title))
            }
        }
        owner_hwnds.push(get_owner_window(hwnd))
    }
    for (hwnd, title) in visiable_hwnds.into_iter() {
        if let Some((i, _)) = owner_hwnds.iter().enumerate().find(|(_, v)| **v == hwnd) {
            if let Some(module_path) = get_module_path(get_window_pid(hwnds[i])) {
                result.entry(module_path).or_default().push((hwnd, title));
            }
        } else if let Some(module_path) = get_module_path(get_window_pid(hwnd)) {
            result.entry(module_path).or_default().push((hwnd, title));
        }
    }
    debug!("list windows {:?}", result);
    Ok(result)
}

extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows: &mut Vec<HWND> = unsafe { &mut *(lparam.0 as *mut _) };
    windows.push(hwnd);
    BOOL(1)
}

pub fn get_uwp_icon_data(module_path: &str) -> Option<Vec<u8>> {
    let logo_path = get_appx_logo_path(module_path)?;
    let mut logo_file = File::open(logo_path).ok()?;
    let mut buffer = vec![];
    logo_file.read_to_end(&mut buffer).ok()?;
    Some(buffer)
}

fn get_appx_logo_path(module_path: &str) -> Option<PathBuf> {
    let module_path = PathBuf::from(module_path);
    let executable = module_path.file_name()?.to_string_lossy();
    let module_dir = module_path.parent()?;
    let manifest_path = module_dir.join("AppxManifest.xml");
    let manifest_file = File::open(manifest_path).ok()?;
    let manifest_file = BufReader::new(manifest_file); // Buffering is important for performance
    let reader = EventReader::new(manifest_file);
    let mut logo_value = None;
    let mut matched = false;
    let mut paths = vec![];
    let mut depth = 0;
    for e in reader {
        match e {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => {
                if paths.len() == depth {
                    paths.push(name.local_name.clone())
                }
                let xpath = paths.join("/");
                if xpath == "Package/Applications/Application" {
                    matched = attributes
                        .iter()
                        .any(|v| v.name.local_name == "Executable" && v.value == executable);
                } else if xpath == "Package/Applications/Application/VisualElements" && matched {
                    if let Some(value) = attributes
                        .iter()
                        .find(|v| {
                            ["Square44x44Logo", "Square30x30Logo", "SmallLogo"]
                                .contains(&v.name.local_name.as_str())
                        })
                        .map(|v| v.value.clone())
                    {
                        logo_value = Some(value);
                        break;
                    }
                }
                depth += 1;
            }
            Ok(XmlEvent::EndElement { .. }) => {
                if paths.len() == depth {
                    paths.pop();
                }
                depth -= 1;
            }
            Err(_) => {
                break;
            }
            _ => {}
        }
    }
    let logo_path = module_dir.join(logo_value?);
    let extension = format!(".{}", logo_path.extension()?.to_string_lossy());
    let logo_path = logo_path.display().to_string();
    let prefix = &logo_path[0..(logo_path.len() - extension.len())];
    for size in [
        "targetsize-256",
        "targetsize-128",
        "targetsize-72",
        "targetsize-36",
        "scale-200",
        "scale-100",
    ] {
        let logo_path = PathBuf::from(format!("{prefix}.{size}{extension}"));
        if logo_path.exists() {
            return Some(logo_path);
        }
    }
    None
}

pub fn create_hicon_from_resource(data: &[u8]) -> Option<HICON> {
    unsafe { CreateIconFromResourceEx(data, TRUE, 0x30000, 100, 100, LR_DEFAULTCOLOR) }
        .ok()
        .or_else(|| unsafe { LoadIconW(None, IDI_APPLICATION) }.ok())
}
