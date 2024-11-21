use anyhow::{Context, Result};
use window_switcher::utils::*;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::Graphics::Dwm::{DWM_CLOAKED_APP, DWM_CLOAKED_INHERITED, DWM_CLOAKED_SHELL};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindow, GW_OWNER};

fn main() -> Result<()> {
    let output = collect_windows_info()?
        .iter()
        .map(|v| v.stringify())
        .collect::<Vec<String>>()
        .join("\n");
    println!("{output}");
    Ok(())
}

#[derive(Debug)]
struct WindowInfo {
    hwnd: HWND,
    title: String,
    owner_hwnd: HWND,
    owner_title: String,
    size: (usize, usize),
    is_visible: bool,
    cloak_type: u32,
    is_iconic: bool,
    is_topmost: bool,
}

impl WindowInfo {
    pub fn stringify(&self) -> String {
        let size = format!("{}x{}", self.size.0, self.size.1);
        format!(
            "visible:{}cloak:{}iconic:{}topmost:{} {:>10} {:>10}:{} {}:{}",
            pretty_bool(self.is_visible),
            pretty_cloak(self.cloak_type),
            pretty_bool(self.is_iconic),
            pretty_bool(self.is_topmost),
            size,
            self.hwnd.0 as isize,
            self.title,
            self.owner_hwnd.0 as isize,
            self.owner_title
        )
    }
}

fn collect_windows_info() -> anyhow::Result<Vec<WindowInfo>> {
    let mut hwnds: Vec<HWND> = Default::default();
    unsafe { EnumWindows(Some(enum_window), LPARAM(&mut hwnds as *mut _ as isize)) }
        .with_context(|| "Fail to enum windows".to_string())?;
    let mut output = vec![];
    for hwnd in hwnds {
        let title = get_window_title(hwnd);
        let cloak_type = get_window_cloak_type(hwnd);
        let is_iconic = is_iconic_window(hwnd);
        let is_topmost = is_topmost_window(hwnd);
        let is_visible = is_visible_window(hwnd);
        let (width, height) = get_window_size(hwnd);
        let owner_hwnd: HWND = unsafe { GetWindow(hwnd, GW_OWNER) }.unwrap_or_default();
        let owner_title = if !owner_hwnd.is_invalid() {
            get_window_title(owner_hwnd)
        } else {
            "".into()
        };
        let window_info = WindowInfo {
            hwnd,
            title,
            owner_hwnd,
            owner_title,
            size: (width as usize, height as usize),
            is_visible,
            cloak_type,
            is_iconic,
            is_topmost,
        };
        output.push(window_info);
    }
    Ok(output)
}

fn pretty_bool(value: bool) -> String {
    if value {
        "✓".into()
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

#[test]
fn test_collect_windows_info() {
    collect_windows_info().unwrap();
}
