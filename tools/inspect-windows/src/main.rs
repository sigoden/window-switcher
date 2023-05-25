use anyhow::{anyhow, Result};
use window_switcher::utils::*;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindow, GW_OWNER};

fn main() -> Result<()> {
    let mut hwnds: Vec<HWND> = Default::default();
    unsafe { EnumWindows(Some(enum_window), LPARAM(&mut hwnds as *mut _ as isize)).ok() }
        .map_err(|e| anyhow!("Fail to get windows {}", e))?;
    for hwnd in hwnds {
        let title = get_window_title(hwnd);
        let is_cloaked = is_cloaked_window(hwnd);
        let is_iconic = is_iconic_window(hwnd);
        let is_topmost = is_topmost_window(hwnd);
        let is_visible = is_visible_window(hwnd);
        let owner_hwnd: HWND = unsafe { GetWindow(hwnd, GW_OWNER) };
        let owner_title = if owner_hwnd.0 > 0 {
            get_window_title(owner_hwnd)
        } else {
            "".into()
        };
        println!(
            "visible:{}cloacked{}iconic{}topmost:{} {}:{} {}:{}",
            pretty_bool(is_visible),
            pretty_bool(is_cloaked),
            pretty_bool(is_iconic),
            pretty_bool(is_topmost),
            hwnd.0,
            title,
            owner_hwnd.0,
            owner_title
        );
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
    let windows: &mut Vec<HWND> = unsafe { &mut *(lparam.0 as *mut _) };
    windows.push(hwnd);
    BOOL(1)
}
