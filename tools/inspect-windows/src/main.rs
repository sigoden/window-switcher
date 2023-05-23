use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use prettytable::{row, Table};
use window_switcher::utils::*;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowPlacement, GetWindowTextW, WINDOWPLACEMENT,
};

type WindowsMap = IndexMap<String, Vec<HWND>>;

fn main() -> Result<()> {
    let mut windows: WindowsMap = Default::default();
    unsafe { EnumWindows(Some(enum_window), LPARAM(&mut windows as *mut _ as isize)).ok() }
        .map_err(|e| anyhow!("Fail to get windows {}", e))?;
    for (module_path, hwnds) in windows {
        println!("{module_path}");
        let mut table = Table::new();
        table.add_row(row![
            "ID",
            "TITLE",
            "SHOW1",
            "SHOW2",
            "SIZE",
            "IS_CLOAKED",
            "IS_ICONIC",
            "IS_POPUP",
            "IS_SPECIAL",
            "IS_TOPMOST",
            "IS_VISIBLE",
        ]);
        for hwnd in hwnds {
            let id = hwnd.0;
            let title = window_title(hwnd);
            let rect = window_rect(hwnd);
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            let is_cloaked = is_cloaked_window(hwnd);
            let is_iconic = is_iconic_window(hwnd);
            let is_popup = is_popup_window(hwnd);
            let is_special = is_special_window(hwnd);
            let is_topmost = is_topmost_window(hwnd);
            let is_visible = is_visible_window(hwnd);
            let is_small = width * height < 5000;
            let size = format!("{width}x{height}{}", pretty_bool(!is_small));
            let show1 = is_visible && !is_special && !is_small && !is_cloaked && !is_popup;
            let show2 = show1 && !is_iconic && !is_topmost;
            table.add_row(row![
                id,
                title,
                pretty_bool(show1),
                pretty_bool(show2),
                size,
                pretty_bool(is_cloaked),
                pretty_bool(is_iconic),
                pretty_bool(is_popup),
                pretty_bool(is_special),
                pretty_bool(is_topmost),
                pretty_bool(is_visible),
            ]);
        }
        table.printstd();
    }
    Ok(())
}

fn window_rect(hwnd: HWND) -> RECT {
    let mut placement = WINDOWPLACEMENT::default();
    unsafe { GetWindowPlacement(hwnd, &mut placement) };
    placement.rcNormalPosition
}

fn window_title(hwnd: HWND) -> String {
    let mut buf = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_slice()) };
    if len == 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..len as usize])
}

fn pretty_bool(value: bool) -> String {
    if value {
        "✓".to_string()
    } else {
        String::new()
    }
}

extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows: &mut WindowsMap = unsafe { &mut *(lparam.0 as *mut _) };
    let pid = get_window_pid(hwnd);
    let module_path = get_module_path(hwnd, pid);
    if module_path.is_empty() {
        return BOOL(1);
    }
    windows.entry(module_path).or_default().push(hwnd);
    BOOL(1)
}
