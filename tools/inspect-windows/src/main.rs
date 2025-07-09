use anyhow::{Context, Result};
use window_switcher::utils::*;

use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::Graphics::Dwm::{DWM_CLOAKED_APP, DWM_CLOAKED_INHERITED, DWM_CLOAKED_SHELL};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindow, GW_OWNER};

fn main() -> Result<()> {
    let mut hwnds: Vec<HWND> = Default::default();
    unsafe { EnumWindows(Some(enum_window), LPARAM(&mut hwnds as *mut _ as isize)) }
        .with_context(|| "Fail to enum windows".to_string())?;
    for hwnd in hwnds {
        let title = get_window_title(hwnd);
        let cloak_type = get_window_cloak_type(hwnd);
        let (is_visible, is_iconic, is_tool, _is_topmost) = get_window_state(hwnd);
        let (width, height) = get_window_size(hwnd);
        let owner_hwnd: HWND = unsafe { GetWindow(hwnd, GW_OWNER) }.unwrap_or_default();
        let owner_title = if !owner_hwnd.is_invalid() {
            get_window_title(owner_hwnd)
        } else {
            "".into()
        };
        println!(
            "visible:{}iconic:{}tool:{}cloak:{} {:>10} {:>10}:{} {}:{}",
            pretty_bool(is_visible),
            pretty_bool(is_iconic),
            pretty_bool(is_tool),
            pretty_cloak(cloak_type),
            format!("{}x{}", width, height),
            hwnd.0 as isize,
            title,
            owner_hwnd.0 as isize,
            owner_title
        );
    }
    Ok(())
}

fn pretty_bool(value: bool) -> String {
    if value {
        "*".into()
    } else {
        " ".into()
    }
}

fn pretty_cloak(value: u32) -> &'static str {
    match value {
        0 => " ",
        DWM_CLOAKED_SHELL => "S",
        DWM_CLOAKED_APP => "A",
        DWM_CLOAKED_INHERITED => "I",
        _ => "?",
    }
}

extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows: &mut Vec<HWND> = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
    windows.push(hwnd);
    BOOL(1)
}
