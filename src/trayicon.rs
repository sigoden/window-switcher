use crate::utils::{wchar_array, wchar_ptr};

use std::{mem::size_of, ptr};
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

const ID_TRAYICON: u32 = 5000;
const ID_TRAYICON_EXIT: u32 = 3000;
const WM_TRAYICON: u32 = WM_USER + 1;

pub fn setup_trayicon() {
    TrayIcon::create()
}

pub struct TrayIcon {
    data: NOTIFYICONDATAW,
    hmenu: HMENU,
    hwnd: HWND,
}

impl TrayIcon {
    pub fn create() {
        unsafe {
            let h_instance = GetModuleHandleW(None);

            debug_assert!(h_instance.0 != 0);

            let wnd_class_name = wchar_ptr("Windows Switcher");
            let wnd_class = WNDCLASSW {
                hInstance: h_instance,
                lpszClassName: wnd_class_name,
                lpfnWndProc: Some(TrayIcon::winproc),
                ..Default::default()
            };
            RegisterClassW(&wnd_class);

            let hmenu = CreatePopupMenu();
            AppendMenuW(
                hmenu,
                MF_STRING,
                ID_TRAYICON_EXIT as usize,
                wchar_ptr("Exit"),
            );
            let trayicon = Self {
                data: Self::gen_data(),
                hwnd: HWND(0),
                hmenu,
            };
            let ptr = Box::into_raw(Box::new(trayicon));

            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                wnd_class_name,
                wchar_ptr("Windows Switcher Hidden Window"),
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
        }
    }

    pub fn add(&mut self) {
        self.data.hWnd = self.hwnd;
        unsafe { Shell_NotifyIconW(NIM_ADD, &self.data) };
    }

    pub fn delete(&mut self) {
        unsafe { Shell_NotifyIconW(NIM_DELETE, &self.data) };
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
                self.add();
            }
            WM_TRAYICON => match lparam.0 as u32 {
                WM_LBUTTONUP | WM_RBUTTONDOWN => unsafe {
                    let mut point = POINT::default();
                    SetForegroundWindow(self.hwnd);
                    GetCursorPos(&mut point);
                    let res = TrackPopupMenu(
                        self.hmenu,
                        TPM_RETURNCMD | TPM_NONOTIFY,
                        point.x,
                        point.y,
                        0,
                        self.hwnd,
                        ptr::null_mut(),
                    );
                    if res.0 as u32 == ID_TRAYICON_EXIT {
                        PostQuitMessage(0);
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
        let icon = unsafe { convert_icon(crate::TRAYICON_ICON_BUFFER) };
        let mut data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            uID: ID_TRAYICON,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: icon,
            ..Default::default()
        };
        wchar_array(crate::TRAYICON_TOOLTIP, data.szTip.as_mut());
        data
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
