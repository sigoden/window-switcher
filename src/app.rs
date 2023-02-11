use crate::startup::Startup;
use crate::switcher::{Switcher, VirtualDesktop};
use crate::trayicon::TrayIcon;
use crate::utils::{
    detect_key_down, get_foreground_window, get_window_exe_name, register_hotkey, unregister_hotkey,
};
use crate::{log_error, log_info, Config, HotKeyConfig, Win32Error};

use anyhow::{anyhow, bail, Result};
use std::ptr::null_mut;
use std::thread;
use std::time::Duration;
use wchar::{wchar_t, wchz};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, PWSTR, WPARAM};
use windows::Win32::System::Com::{CoInitialize, CoUninitialize};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, PostQuitMessage,
    RegisterClassW, RegisterWindowMessageW, SendMessageW, TranslateMessage, CREATESTRUCTW,
    CW_USEDEFAULT, GWL_USERDATA, MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WM_COMMAND, WM_CREATE,
    WM_HOTKEY, WM_LBUTTONUP, WM_RBUTTONUP, WM_USER, WNDCLASSW,
};

pub const WM_MISC_TRAYICON: u32 = WM_USER + 1; // trayicon click
pub const WM_MISC_KEYUP: u32 = WM_USER + 2; // hot key release
pub const WM_MISC_WINDOW: u32 = WM_USER + 3; // foreground window change
pub const MENU_CMD_EXIT: u32 = 1;
pub const MENU_CMD_STARTUP: u32 = 2;

pub const NAME: &[wchar_t] = wchz!("Windows Switcher");

pub fn start_app(config: &Config) {
    if let Err(err) = App::start(config) {
        log_error!(&err.to_string());
    }
}

pub struct App {
    trayicon: Option<TrayIcon>,
    startup: Startup,
    hwnd: HWND,
    msg_cb: Option<u32>,
    has_com: bool,
    switcher: Switcher,
    config: Config,
    keys_up: [bool; 2],
    hotkeys_enable: [bool; 2],
}

impl App {
    pub fn start(config: &Config) -> Result<()> {
        log_info!("App::start config={:?}", config);
        let has_com = match Self::init_com() {
            Ok(_) => true,
            Err(err) => {
                log_error!(err.to_string());
                false
            }
        };
        let virtual_desktop = match VirtualDesktop::create() {
            Ok(v) => Some(v),
            Err(err) => {
                log_error!(err.to_string());
                None
            }
        };
        let instance = Self::get_instance()?;

        let name = PWSTR(NAME.as_ptr());

        Self::register_window_class(instance, name)?;

        let trayicon = match config.trayicon {
            true => Some(TrayIcon::create()),
            false => None,
        };
        let startup = Startup::create()?;
        let switcher = Switcher::new(virtual_desktop);

        let app = App {
            trayicon,
            startup,
            hwnd: HWND::default(),
            msg_cb: None,
            has_com,
            switcher,
            config: config.clone(),
            keys_up: Default::default(),
            hotkeys_enable: Default::default(),
        };

        let hwnd = Self::create_window(instance, name, app)?;

        let empty_blacklist = config.blacklist.is_empty();

        if empty_blacklist {
            register_hotkey(hwnd, 0, &config.hotkeys[0])?;
        }

        if let Err(err) = register_hotkey(hwnd, 1, &config.hotkeys[1]) {
            log_error!("{}", err);
        }

        let hotkeys = config.hotkeys.clone();
        thread::spawn(move || {
            let mut is_key_down_prev = false;
            let mut is_key2_down_prev = false;
            let mut fg_hwnd_prev = HWND::default();
            let watch_key = |id: usize, hotkey: &HotKeyConfig, is_key_down_prev: &mut bool| {
                match (*is_key_down_prev, detect_key_down(hotkey.meta)) {
                    (true, false) => {
                        // alt key release
                        *is_key_down_prev = false;
                        unsafe { SendMessageW(hwnd, WM_MISC_KEYUP, WPARAM(id), LPARAM(0)) };
                    }
                    (false, true) => {
                        *is_key_down_prev = true;
                    }
                    _ => {}
                }
            };

            loop {
                thread::sleep(Duration::from_millis(100));
                if !empty_blacklist {
                    let fg_hwnd = get_foreground_window();
                    if fg_hwnd != fg_hwnd_prev {
                        unsafe { SendMessageW(hwnd, WM_MISC_WINDOW, WPARAM(0), LPARAM(0)) };
                        fg_hwnd_prev = fg_hwnd;
                    }
                }
                watch_key(0, &hotkeys[0], &mut is_key_down_prev);
                if hotkeys[0].meta != hotkeys[1].meta {
                    watch_key(1, &hotkeys[1], &mut is_key2_down_prev);
                }
            }
        });

        Self::eventloop();

        App::destory(hwnd);

        Ok(())
    }

    fn init_com() -> Result<()> {
        unsafe { CoInitialize(null_mut()).map_err(|e| anyhow!("Fail to init com, {}", e)) }
    }

    fn destory(hwnd: HWND) {
        unsafe { std::ptr::drop_in_place(get_window_ptr(hwnd) as *mut Self) }
    }

    fn get_instance() -> Result<HINSTANCE> {
        let instance = unsafe { GetModuleHandleW(None) }
            .ok()
            .map_err(|e| anyhow!("Fail to get instance, {}", e))?;

        debug_assert!(instance.0 != 0);
        Ok(instance)
    }

    fn register_window_class(instance: HINSTANCE, name: PWSTR) -> Result<()> {
        let class = WNDCLASSW {
            hInstance: instance,
            lpszClassName: name,
            lpfnWndProc: Some(App::winproc),
            ..Default::default()
        };
        let atom = unsafe { RegisterClassW(&class) };
        if atom == 0 {
            bail!("Fail to register class, {}", Win32Error::from_win32());
        }
        Ok(())
    }

    fn create_window(instance: HINSTANCE, name: PWSTR, app: App) -> Result<HWND> {
        let app_ptr = Box::into_raw(Box::new(app));
        unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                name,
                name,
                WINDOW_STYLE(0),
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                instance,
                app_ptr as *mut _,
            )
        }
        .ok()
        .map_err(|e| anyhow!("Fail to create window, {}", e))
    }

    fn eventloop() {
        let mut message = MSG::default();
        loop {
            let ret = unsafe { GetMessageW(&mut message, HWND(0), 0, 0) };
            match ret.0 {
                0 => break,
                -1 => {
                    log_error!("Fail to get message, {}", Win32Error::from_win32());
                }
                _ => unsafe {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                },
            }
        }
    }

    unsafe extern "system" fn winproc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match Self::handle_wm(hwnd, msg, wparam, lparam) {
            Ok(ret) => ret,
            Err(err) => {
                log_error!(&err.to_string());
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
    }

    fn handle_wm(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Result<LRESULT> {
        match msg {
            WM_CREATE => unsafe {
                log_info!("Handle msg=WM_CREATE");
                let create_struct: &mut CREATESTRUCTW = &mut *(lparam.0 as *mut _);
                let app: &mut App = &mut *(create_struct.lpCreateParams as *mut _);
                set_window_ptr(hwnd, app);
                app.hwnd = hwnd;
                if let Some(trayicon) = app.trayicon.as_mut() {
                    trayicon.add(hwnd)?;
                }
                app.msg_cb = {
                    Some(RegisterWindowMessageW(PWSTR(
                        wchz!("TaskbarCreated").as_ptr(),
                    )))
                };
            },
            WM_HOTKEY => {
                let hotkey_id = wparam.0;
                log_info!("Handle msg=WM_HOTKEY, hotkey_id={}", hotkey_id);
                let app = retrive_app(hwnd)?;
                if hotkey_id == 0 {
                    app.switcher.switch_window(app.keys_up[hotkey_id])?;
                } else if hotkey_id == 1 {
                    app.switcher.switch_app(app.keys_up[hotkey_id])?;
                }
                app.keys_up[hotkey_id] = false;
            }
            WM_MISC_TRAYICON => {
                let app = retrive_app(hwnd)?;
                if let Some(trayicon) = app.trayicon.as_mut() {
                    let keycode = lparam.0 as u32;
                    if keycode == WM_LBUTTONUP || keycode == WM_RBUTTONUP {
                        log_info!("Handle msg=WM_MISC_TRAYICON");
                        trayicon.popup(app.startup.is_enable)?;
                    }
                }
                return Ok(LRESULT(0));
            }
            WM_MISC_KEYUP => {
                log_info!("Handle msg=WM_MISC_KEYUP, hotkey_id={}", wparam.0);
                let app = retrive_app(hwnd)?;
                let hotkey_id = wparam.0;
                if hotkey_id == 0 && app.config.hotkeys[0].meta == app.config.hotkeys[1].meta {
                    app.keys_up[1] = true;
                }
                app.keys_up[hotkey_id] = true;
            }
            WM_MISC_WINDOW => {
                let app = retrive_app(hwnd)?;
                let hotkey_id = 0;
                let hotkey = &app.config.hotkeys[hotkey_id];
                let name = get_window_exe_name(get_foreground_window());
                if !name.is_empty() {
                    let is_black = app.config.blacklist.contains(&format!(",{name}"));
                    match (is_black, app.hotkeys_enable[hotkey_id]) {
                        (true, true) => {
                            if let Err(err) = unregister_hotkey(hwnd, hotkey_id) {
                                log_error!("{}", err);
                            }
                            app.hotkeys_enable[hotkey_id] = false;
                        }
                        (false, false) => {
                            if let Err(err) = register_hotkey(hwnd, hotkey_id, hotkey) {
                                log_error!("{}", err);
                            }
                            app.hotkeys_enable[hotkey_id] = true;
                        }
                        _ => {}
                    }
                }
            }
            WM_COMMAND => {
                let value = wparam.0 as u32;
                let kind = ((value >> 16) & 0xffff) as u16;
                let id = value & 0xffff;
                if kind == 0 {
                    match id {
                        MENU_CMD_EXIT => {
                            log_info!("Handle msg=MENU_CMD_EXIT");
                            unsafe { PostQuitMessage(0) };
                        }
                        MENU_CMD_STARTUP => {
                            log_info!("Handle msg=MENU_CMD_STARTUP");
                            let app = retrive_app(hwnd)?;
                            app.startup.toggle()?;
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                if let Ok(app) = retrive_app(hwnd) {
                    if let Some(msg_id) = app.msg_cb {
                        if msg == msg_id {
                            if let Some(trayicon) = app.trayicon.as_mut() {
                                trayicon.add(hwnd)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) })
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if self.has_com {
            unsafe { CoUninitialize() }
        }
    }
}

fn retrive_app(hwnd: HWND) -> Result<&'static mut App> {
    unsafe {
        let ptr = get_window_ptr(hwnd);
        if ptr == 0 {
            bail!("Fail to retrieve app from window ptr");
        }
        let tx: &mut App = &mut *(ptr as *mut _);
        Ok(tx)
    }
}

#[cfg(target_arch = "x86")]
fn get_window_ptr(hwnd: HWND) -> i32 {
    unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowLongW(hwnd, GWL_USERDATA) }
}
#[cfg(target_arch = "x86_64")]
fn get_window_ptr(hwnd: HWND) -> isize {
    unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd, GWL_USERDATA) }
}

#[cfg(target_arch = "x86")]
fn set_window_ptr(hwnd: HWND, app: &mut App) {
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::SetWindowLongW(
            hwnd,
            GWL_USERDATA,
            app as *mut _ as _,
        )
    };
}

#[cfg(target_arch = "x86_64")]
fn set_window_ptr(hwnd: HWND, app: &mut App) {
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
            hwnd,
            GWL_USERDATA,
            app as *mut _ as _,
        )
    };
}
