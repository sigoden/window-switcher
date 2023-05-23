use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use window_switcher::utils::*;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::EnumWindows;

type WindowsMap = IndexMap<String, Vec<HWND>>;

fn main() -> Result<()> {
    let mut windows: WindowsMap = Default::default();
    unsafe { EnumWindows(Some(enum_window), LPARAM(&mut windows as *mut _ as isize)).ok() }
        .map_err(|e| anyhow!("Fail to get windows {}", e))?;
    for (module_path, hwnds) in windows {
        println!("{module_path}");
        for hwnd in hwnds {
            let title = get_window_title(hwnd);
            let rect = get_window_rect(hwnd);
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            let is_cloaked = is_cloaked_window(hwnd);
            let is_iconic = is_iconic_window(hwnd);
            let is_popup = is_popup_window(hwnd);
            let is_special = is_special_window(hwnd);
            let is_topmost = is_topmost_window(hwnd);
            let is_visible = is_visible_window(hwnd);
            if is_small_window(hwnd) {
                continue;
            }
            let is_show = is_show_window(hwnd);
            let size = format!("{width}x{height}");
            println!(
                "cloacked{}iconic{}popup:{}special:{}topmost:{}visible:{}show:{} {}:{}",
                pretty_bool(is_cloaked),
                pretty_bool(is_iconic),
                pretty_bool(is_popup),
                pretty_bool(is_special),
                pretty_bool(is_topmost),
                pretty_bool(is_visible),
                pretty_bool(is_show),
                title,
                size,
            );
        }
    }
    Ok(())
}

fn pretty_bool(value: bool) -> String {
    if value {
        "✓".into()
    } else {
        " ".into()
    }
}

extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows: &mut WindowsMap = unsafe { &mut *(lparam.0 as *mut _) };
    let pid = get_window_pid(hwnd);
    let module_path = match get_module_path(hwnd, pid) {
        Some(v) => v,
        None => return BOOL(1),
    };
    windows.entry(module_path).or_default().push(hwnd);
    BOOL(1)
}
