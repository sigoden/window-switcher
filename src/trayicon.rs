use crate::app::{IDM_EXIT, IDM_STARTUP, NAME, WM_USER_TRAYICON};

use anyhow::{anyhow, Result};
use windows::core::w;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconFromResourceEx, CreatePopupMenu, GetCursorPos,
    LookupIconIdFromDirectoryEx, SetForegroundWindow, TrackPopupMenu, HMENU, LR_DEFAULTCOLOR,
    MF_CHECKED, MF_STRING, MF_UNCHECKED, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
};

const ICON_BYTES: &[u8] = include_bytes!("../assets/icon.ico");
const TEXT_STARTUP: PCWSTR = w!("Startup");
const TEXT_EXIT: PCWSTR = w!("Exit");

pub struct TrayIcon {
    data: NOTIFYICONDATAW,
}

impl TrayIcon {
    pub fn create() -> Self {
        let data = Self::create_nid();
        Self { data }
    }

    pub fn register(&mut self, hwnd: HWND) -> Result<()> {
        self.data.hWnd = hwnd;
        unsafe { Shell_NotifyIconW(NIM_ADD, &self.data) }
            .ok()
            .map_err(|e| anyhow!("Fail to add trayicon, {}", e))
    }

    pub fn show(&mut self, startup: bool) -> Result<()> {
        let hwnd = self.data.hWnd;
        let mut cursor = POINT::default();
        unsafe {
            SetForegroundWindow(hwnd)
                .ok()
                .map_err(|e| anyhow!("Fail to set foreground window, {}", e))?;
            GetCursorPos(&mut cursor).map_err(|e| anyhow!("Fail to get cursor pos, {}", e))?;
            let hmenu = self
                .create_menu(startup)
                .map_err(|e| anyhow!("Fail to create menu, {}", e))?;
            TrackPopupMenu(
                hmenu,
                TPM_LEFTALIGN | TPM_BOTTOMALIGN,
                cursor.x,
                cursor.y,
                0,
                hwnd,
                None,
            )
            .ok()
            .map_err(|e| anyhow!("Fail to show popup menu, {}", e))?
        };
        Ok(())
    }

    fn create_nid() -> NOTIFYICONDATAW {
        let offset = unsafe {
            LookupIconIdFromDirectoryEx(ICON_BYTES.as_ptr(), true, 0, 0, LR_DEFAULTCOLOR)
        };
        let icon_data = &ICON_BYTES[offset as usize..];
        let hicon =
            unsafe { CreateIconFromResourceEx(icon_data, true, 0x30000, 0, 0, LR_DEFAULTCOLOR) }
                .expect("Failed to load icon resource");
        let mut tooltip: Vec<u16> = unsafe { NAME.as_wide() }.to_vec();
        tooltip.resize(128, 0);
        tooltip.pop();
        tooltip.push(0);
        let tooltip: [u16; 128] = tooltip.try_into().unwrap();
        NOTIFYICONDATAW {
            uID: WM_USER_TRAYICON,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_USER_TRAYICON,
            hIcon: hicon,
            szTip: tooltip,
            ..Default::default()
        }
    }

    fn create_menu(&mut self, startup: bool) -> Result<HMENU> {
        let startup_flags = if startup { MF_CHECKED } else { MF_UNCHECKED };
        unsafe {
            let hmenu = CreatePopupMenu().map_err(|err| anyhow!("Failed to create menu, {err}"))?;
            AppendMenuW(hmenu, startup_flags, IDM_STARTUP as usize, TEXT_STARTUP)?;
            AppendMenuW(hmenu, MF_STRING, IDM_EXIT as usize, TEXT_EXIT)?;
            Ok(hmenu)
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        debug!("trayicon destoryed");
        unsafe { Shell_NotifyIconW(NIM_DELETE, &self.data) };
    }
}
