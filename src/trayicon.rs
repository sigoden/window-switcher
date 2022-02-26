use crate::startup::Startup;
use crate::utils::{output_debug, wchar_array, wchar_ptr};

use std::{mem::size_of, ptr};
use windows::Win32::Foundation::GetLastError;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconFromResourceEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
    GetCursorPos, GetWindowLongPtrW, LookupIconIdFromDirectoryEx, PostQuitMessage, RegisterClassW,
    SetForegroundWindow, SetWindowLongPtrW, TrackPopupMenu, CREATESTRUCTW, CW_USEDEFAULT,
    GWL_USERDATA, HICON, HMENU, LR_DEFAULTCOLOR, MF_STRING, TPM_NONOTIFY, TPM_RETURNCMD,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_CREATE, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_USER, WNDCLASSW,
};
use windows::{
    Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
    Win32::System::LibraryLoader::GetModuleHandleW,
    Win32::UI::Shell::{
        Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
    },
};

const ID_EXIT: usize = 3000;
const ID_STARTUP: usize = 3001;
const ID_TRAYICON: u32 = 5000;
const WM_TRAYICON: u32 = WM_USER + 1;
const TRAYICON_TOOLTIP: &str = "Windows Swither On";
const TRAYICON_ICON_BUFFER: &[u8] = include_bytes!("../assets/icon.ico");

pub fn setup_trayicon() {
    TrayIcon::create();
}

pub struct TrayIcon {
    data: NOTIFYICONDATAW,
    hwnd: HWND,
    startup: Startup,
}

impl TrayIcon {
    pub fn create() -> bool {
        unsafe {
            let h_instance = GetModuleHandleW(None);
            if h_instance.is_invalid() {
                let err = GetLastError();
                output_debug(&format!("TrayIcon: Fail to get module handle, {:?}", err));
                return false;
            }

            debug_assert!(h_instance.0 != 0);

            let wnd_class_name = wchar_ptr("Windows Switcher");
            let wnd_class = WNDCLASSW {
                hInstance: h_instance,
                lpszClassName: wnd_class_name,
                lpfnWndProc: Some(TrayIcon::winproc),
                ..Default::default()
            };
            let atom = RegisterClassW(&wnd_class);
            if atom == 0 {
                let err = GetLastError();
                output_debug(&format!("TrayIcon: Fail to register class, {:?}", err));
                return false;
            }

            let trayicon = Self {
                data: Self::gen_data(),
                hwnd: HWND(0),
                startup: Startup::default(),
            };
            let ptr = Box::into_raw(Box::new(trayicon));

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                wnd_class_name,
                wnd_class_name,
                WINDOW_STYLE(0),
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                h_instance,
                ptr as *mut _,
            );
            if hwnd.is_invalid() {
                let err = GetLastError();
                output_debug(&format!("TrayIcon: Fail to create window, {:?}", err));
                return false;
            }
            true
        }
    }

    pub fn add(&mut self) {
        self.data.hWnd = self.hwnd;
        let ret = unsafe { Shell_NotifyIconW(NIM_ADD, &self.data) };
        if !ret.as_bool() {
            output_debug("TrayIcon: Fail to add trayicon");
        }
    }

    pub fn delete(&mut self) {
        let ret = unsafe { Shell_NotifyIconW(NIM_DELETE, &self.data) };
        if !ret.as_bool() {
            output_debug("TrayIon: Fail to delete trayicon");
        }
    }

    pub unsafe extern "system" fn winproc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_CREATE => {
                let create_struct: &mut CREATESTRUCTW = &mut *(lparam.0 as *mut _);
                let trayicon: &mut Self = &mut *(create_struct.lpCreateParams as *mut _);
                trayicon.hwnd = hwnd;
                SetWindowLongPtrW(hwnd, GWL_USERDATA, trayicon as *mut _ as _);
                trayicon.add();
                trayicon.handle(msg, wparam, lparam)
            }
            _ => {
                let winptr = GetWindowLongPtrW(hwnd, GWL_USERDATA);
                if winptr != 0 {
                    let trayicon: &mut Self = &mut *(winptr as *mut _);
                    trayicon.handle(msg, wparam, lparam)
                } else {
                    DefWindowProcW(hwnd, msg, wparam, lparam)
                }
            }
        }
    }
    fn handle(&mut self, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_CREATE => {
                output_debug("Trayicon: Handle WM_CREATE");
                self.add();
            }
            WM_TRAYICON => match lparam.0 as u32 {
                WM_LBUTTONUP | WM_RBUTTONDOWN => unsafe {
                    let mut point = POINT::default();
                    SetForegroundWindow(self.hwnd);
                    GetCursorPos(&mut point);
                    let hmenu = self.create_menu();
                    let res = TrackPopupMenu(
                        &hmenu,
                        TPM_RETURNCMD | TPM_NONOTIFY,
                        point.x,
                        point.y,
                        0,
                        self.hwnd,
                        ptr::null_mut(),
                    );
                    match res.0 as usize {
                        ID_EXIT => {
                            PostQuitMessage(0);
                        }
                        ID_STARTUP => {
                            self.startup.toggle();
                        }
                        _ => {}
                    }
                    return LRESULT(0);
                },
                _ => {}
            },
            _ => {}
        }
        unsafe { DefWindowProcW(self.hwnd, msg, wparam, lparam) }
    }
    fn gen_data() -> NOTIFYICONDATAW {
        let icon = unsafe { convert_icon(TRAYICON_ICON_BUFFER) };
        let mut data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            uID: ID_TRAYICON,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: icon,
            ..Default::default()
        };
        wchar_array(TRAYICON_TOOLTIP, data.szTip.as_mut());
        data
    }
    fn create_menu(&mut self) -> HMENU {
        let text = {
            if self.startup.check() {
                wchar_ptr("Startup: on")
            } else {
                wchar_ptr("Startup: off")
            }
        };
        unsafe {
            let hmenu = CreatePopupMenu();
            AppendMenuW(hmenu, MF_STRING, ID_STARTUP, text);
            AppendMenuW(hmenu, MF_STRING, ID_EXIT, wchar_ptr("Exit"));
            hmenu
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        self.delete();
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
