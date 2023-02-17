use crate::startup::Startup;
use crate::switcher::Switcher;
use crate::trayicon::TrayIcon;
use crate::utils::{
    check_error, get_foreground_window, get_window_exe, get_window_ptr, register_hotkey,
    set_window_ptr, unregister_hotkey, CheckError,
};
use crate::{alert, debug, error, Config, HotKeyConfig};

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use std::thread;
use std::time::Duration;
use windows::core::PCWSTR;
use windows::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, PostQuitMessage,
    RegisterClassW, RegisterWindowMessageW, SendMessageW, TranslateMessage, CREATESTRUCTW,
    CW_USEDEFAULT, MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WM_COMMAND, WM_CREATE, WM_HOTKEY,
    WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSW,
};

pub const WM_USER_TRAYICON: u32 = 6000;
pub const WM_USER_MODIFIER_KEYUP: u32 = 6001;
pub const WM_USER_FOREGROUND_CHANGE: u32 = 6002;
pub const IDM_EXIT: u32 = 1;
pub const IDM_STARTUP: u32 = 2;

pub const NAME: PCWSTR = w!("Windows Switcher");

pub fn start(config: &Config) {
    if let Err(err) = App::start(config) {
        alert!("{err}");
    }
}

/// When the taskbar is created, it registers a message with the "TaskbarCreated" string and then broadcasts this message to all top-level windows
/// When the application receives this message, it should assume that any taskbar icons it added have been removed and add them again.
static S_U_TASKBAR_RESTART: Lazy<u32> =
    Lazy::new(|| unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) });

pub struct App {
    hwnd: HWND,
    trayicon: Option<TrayIcon>,
    startup: Startup,
    switcher: Switcher,
    config: Config,
    enable_hotkey: bool,
    modifier_keyup: bool,
}

impl App {
    pub fn start(config: &Config) -> Result<()> {
        debug!("App::start config={:?}", config);

        let hinstance = unsafe { GetModuleHandleW(None) }
            .map_err(|err| anyhow!("Failed to get current module handle, {err}"))?;

        let window_class = WNDCLASSW {
            hInstance: hinstance,
            lpszClassName: NAME,
            lpfnWndProc: Some(App::window_proc),
            ..Default::default()
        };

        let atom = unsafe { RegisterClassW(&window_class) }
            .check_error()
            .map_err(|err| anyhow!("Failed to register class, {err}"))?;

        let trayicon = match config.trayicon {
            true => Some(TrayIcon::create()),
            false => None,
        };
        let startup = Startup::init()?;
        let switcher = Switcher::new();
        let is_empty_blacklist = config.blacklist.is_empty();

        let app = App {
            hwnd: HWND::default(),
            trayicon,
            startup,
            switcher,
            config: config.clone(),
            modifier_keyup: true,
            enable_hotkey: is_empty_blacklist,
        };

        let app_ptr = Box::into_raw(Box::new(app));

        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR(atom as *mut u16),
                NAME,
                WINDOW_STYLE(0),
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                hinstance,
                Some(app_ptr as *const _),
            )
        }
        .check_error()
        .map_err(|err| anyhow!("Failed to create windows, {err}"))?;

        let hotkey = config.hotkey.clone();
        if is_empty_blacklist {
            register_hotkey(hwnd, &hotkey)?;
        }
        thread::spawn(move || {
            watch(hwnd, hotkey, is_empty_blacklist);
        });

        Self::eventloop()
    }

    fn eventloop() -> Result<()> {
        let mut message = MSG::default();
        loop {
            let ret = unsafe { GetMessageW(&mut message, HWND(0), 0, 0) };
            match ret.0 {
                -1 => {}
                0 => break,
                _ => unsafe {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                },
            }
        }

        Ok(())
    }

    fn set_trayicon(&mut self) -> Result<()> {
        if let Some(trayicon) = self.trayicon.as_mut() {
            trayicon.register(self.hwnd)?;
        }
        Ok(())
    }

    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match Self::handle_message(hwnd, msg, wparam, lparam) {
            Ok(ret) => ret,
            Err(err) => {
                error!("{err}");
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
    }

    fn handle_message(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Result<LRESULT> {
        match msg {
            WM_USER_TRAYICON => {
                let app = retrive_app(hwnd)?;
                if let Some(trayicon) = app.trayicon.as_mut() {
                    let keycode = lparam.0 as u32;
                    if keycode == WM_LBUTTONUP || keycode == WM_RBUTTONUP {
                        trayicon.show(app.startup.is_enable)?;
                    }
                }
                return Ok(LRESULT(0));
            }
            WM_USER_MODIFIER_KEYUP => {
                let app = retrive_app(hwnd)?;
                app.modifier_keyup = true;
            }
            WM_USER_FOREGROUND_CHANGE => {
                let fg_hwnd = get_foreground_window();
                let exe = get_window_exe(fg_hwnd);
                if exe.is_empty() {
                    return Ok(LRESULT(0));
                };
                let app = retrive_app(hwnd)?;
                let config = &app.config;
                match (config.blacklist.contains(&exe), app.enable_hotkey) {
                    (false, false) => match register_hotkey(hwnd, &config.hotkey) {
                        Ok(_) => app.enable_hotkey = true,
                        Err(err) => error!("{err}"),
                    },
                    (true, true) => match unregister_hotkey(hwnd) {
                        Ok(_) => app.enable_hotkey = false,
                        Err(err) => error!("{err}"),
                    },
                    _ => {}
                }
            }
            WM_CREATE => {
                debug!("Handle msg=WM_CREATE");
                let app: &mut App = unsafe {
                    let create_struct: &mut CREATESTRUCTW = &mut *(lparam.0 as *mut _);
                    set_window_ptr(hwnd, create_struct.lpCreateParams as _);
                    &mut *(create_struct.lpCreateParams as *mut _)
                };
                app.hwnd = hwnd;
                app.set_trayicon()?;
            }
            WM_HOTKEY => {
                debug!("Handle msg=WM_HOTKEY");
                let app = retrive_app(hwnd)?;
                app.switcher.next_window(app.modifier_keyup)?;
                app.modifier_keyup = false;
            }
            WM_COMMAND => {
                let value = wparam.0 as u32;
                let kind = ((value >> 16) & 0xffff) as u16;
                let id = value & 0xffff;
                if kind == 0 {
                    match id {
                        IDM_EXIT => {
                            if let Ok(app) = retrive_app(hwnd) {
                                unsafe { drop(Box::from_raw(app)) }
                            }
                            unsafe { PostQuitMessage(0) }
                        }
                        IDM_STARTUP => {
                            let app = retrive_app(hwnd)?;
                            app.startup.toggle()?;
                        }
                        _ => {}
                    }
                }
            }
            _ if msg == *S_U_TASKBAR_RESTART => {
                let app = retrive_app(hwnd)?;
                app.set_trayicon()?;
            }
            _ => {}
        }
        Ok(unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) })
    }
}

fn retrive_app(hwnd: HWND) -> Result<&'static mut App> {
    unsafe {
        let ptr = check_error(|| get_window_ptr(hwnd))
            .map_err(|err| anyhow!("Failed to get window ptr, {err}"))?;
        let tx: &mut App = &mut *(ptr as *mut _);
        Ok(tx)
    }
}

pub fn watch(hwnd: HWND, hotkey: HotKeyConfig, is_empty_blacklist: bool) {
    let mut fg_hwnd_prev = HWND::default();
    let mut is_modifer_pressed_prev: bool = false;
    loop {
        thread::sleep(Duration::from_millis(100));
        if !is_empty_blacklist {
            let fg_hwnd = get_foreground_window();
            if fg_hwnd != fg_hwnd_prev {
                unsafe { SendMessageW(hwnd, WM_USER_FOREGROUND_CHANGE, WPARAM(0), LPARAM(0)) };
                fg_hwnd_prev = fg_hwnd;
            }
        }

        let is_modifer_pressed = unsafe { GetKeyState(hotkey.modifier.0.into()) } < 0;
        match (is_modifer_pressed_prev, is_modifer_pressed) {
            (true, false) => {
                is_modifer_pressed_prev = false;
                unsafe { SendMessageW(hwnd, WM_USER_MODIFIER_KEYUP, WPARAM(0), LPARAM(0)) };
            }
            (false, true) => {
                is_modifer_pressed_prev = true;
            }
            _ => {}
        }
    }
}
