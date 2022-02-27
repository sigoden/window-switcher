use crate::app::{MENU_CMD_EXIT, MENU_CMD_STARTUP, NAME, WM_USER_TRAYICON};
use crate::Win32Result;

use anyhow::{anyhow, Result};
use std::{mem::size_of, ptr};
use wchar::{wchar_t, wchz};
use windows::Win32::Foundation::{HWND, POINT, PWSTR};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconFromResourceEx, CreatePopupMenu, DestroyMenu, GetCursorPos,
    LookupIconIdFromDirectoryEx, SetForegroundWindow, TrackPopupMenu, HICON, HMENU,
    LR_DEFAULTCOLOR, MF_STRING, TRACK_POPUP_MENU_FLAGS,
};

const TRAYICON_ICON_BUFFER: &[u8] = include_bytes!("../assets/icon.ico");
const TEXT_STARTUP_ON: &[wchar_t] = wchz!("Startup: On");
const TEXT_STARTUP_OFF: &[wchar_t] = wchz!("Startup: Off");
const TEXT_EXIT: &[wchar_t] = wchz!("Exit");

pub struct TrayIcon {
    data: NOTIFYICONDATAW,
}

impl TrayIcon {
    pub fn create() -> Self {
        let data = Self::gen_data();
        Self { data }
    }
    pub fn add(&mut self, hwnd: HWND) -> Result<()> {
        self.data.hWnd = hwnd;
        unsafe { Shell_NotifyIconW(NIM_ADD, &self.data) }
            .ok()
            .map_err(|e| anyhow!("Fail to add trayicon, {}", e))
    }
    pub fn popup(&mut self, startup: bool) -> Result<()> {
        let hwnd = self.hwnd();
        let mut point = POINT::default();
        unsafe {
            SetForegroundWindow(hwnd)
                .ok()
                .map_err(|e| anyhow!("Fail to set foreground window, {}", e))?;
            GetCursorPos(&mut point)
                .ok()
                .map_err(|e| anyhow!("Fail to get cursor pos, {}", e))?;
            let menu = self
                .create_menu(startup)
                .map_err(|e| anyhow!("Fail to create menu, {}", e))?;

            TrackPopupMenu(
                menu.hmenu,
                TRACK_POPUP_MENU_FLAGS::default(),
                point.x,
                point.y,
                0,
                hwnd,
                ptr::null_mut(),
            )
            .ok()
            .map_err(|e| anyhow!("Fail to show popup menu, {}", e))?
        };
        Ok(())
    }
    fn hwnd(&self) -> HWND {
        self.data.hWnd
    }
    fn gen_data() -> NOTIFYICONDATAW {
        let icon = unsafe { convert_icon(TRAYICON_ICON_BUFFER) };
        let mut data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            uID: WM_USER_TRAYICON,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_USER_TRAYICON,
            hIcon: icon,
            ..Default::default()
        };
        let tip = data.szTip.as_mut();
        tip[..NAME.len()].copy_from_slice(NAME);
        data
    }
    fn create_menu(&mut self, startup: bool) -> Win32Result<WrapHMenu> {
        let text_startup = match startup {
            true => TEXT_STARTUP_ON,
            false => TEXT_STARTUP_OFF,
        };
        unsafe {
            let hmenu = CreatePopupMenu();
            AppendMenuW(
                hmenu,
                MF_STRING,
                MENU_CMD_STARTUP as usize,
                PWSTR(text_startup.as_ptr()),
            )
            .ok()?;
            AppendMenuW(
                hmenu,
                MF_STRING,
                MENU_CMD_EXIT as usize,
                PWSTR(TEXT_EXIT.as_ptr()),
            )
            .ok()?;
            Ok(WrapHMenu { hmenu })
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        unsafe { Shell_NotifyIconW(NIM_DELETE, &self.data) };
    }
}

struct WrapHMenu {
    hmenu: HMENU,
}

impl Drop for WrapHMenu {
    fn drop(&mut self) {
        unsafe { DestroyMenu(&self.hmenu) };
    }
}

unsafe fn convert_icon(buffer: &[u8]) -> HICON {
    let offset = { LookupIconIdFromDirectoryEx(buffer.as_ptr(), true, 0, 0, LR_DEFAULTCOLOR) };
    let icon_data = &buffer[offset as usize..];
    CreateIconFromResourceEx(
        icon_data.as_ptr(),
        icon_data.len() as u32,
        true,
        0x30000,
        0,
        0,
        LR_DEFAULTCOLOR,
    )
}
