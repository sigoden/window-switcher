use crate::config::Config;
use crate::foregound::ForegroundWatcher;
use crate::keyboard::KeyboardListener;
use crate::startup::Startup;
use crate::trayicon::TrayIcon;
use crate::utils::{
    check_error, create_hicon_from_resource, get_foreground_window, get_module_icon,
    get_module_path, get_uwp_icon_data, get_window_pid, get_window_user_data, list_windows,
    set_foregound_window, set_window_user_data, CheckError,
};

use anyhow::{anyhow, Result};
use indexmap::IndexSet;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use windows::core::PCWSTR;
use windows::w;
use windows::Win32::Foundation::{
    GetLastError, COLORREF, HMODULE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, EndPaint, FillRect, GetMonitorInfoW, MonitorFromPoint,
    RedrawWindow, HRGN, MONITORINFO, MONITOR_DEFAULTTONEAREST, PAINTSTRUCT, RDW_ERASE,
    RDW_INVALIDATE,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyIcon, DispatchMessageW, DrawIconEx, GetCursorPos,
    GetMessageW, GetWindowLongPtrW, LoadCursorW, PostQuitMessage, RegisterClassW,
    RegisterWindowMessageW, SetCursor, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    TranslateMessage, CW_USEDEFAULT, DI_NORMAL, GWL_STYLE, HICON, HWND_TOPMOST, IDC_ARROW, MSG,
    SWP_SHOWWINDOW, SW_HIDE, WINDOW_STYLE, WM_COMMAND, WM_LBUTTONUP, WM_PAINT, WM_RBUTTONUP,
    WNDCLASSW, WS_CAPTION, WS_EX_TOOLWINDOW,
};

pub const NAME: PCWSTR = w!("Window Switcher");
pub const WM_USER_TRAYICON: u32 = 6000;
pub const WM_USER_MODIFIER_KEYUP: u32 = 6001;
pub const WM_USER_HOOTKEY: u32 = 6002;
pub const IDM_EXIT: u32 = 1;
pub const IDM_STARTUP: u32 = 2;

const BG_COLOR: COLORREF = COLORREF(0x4c4c4c);
const FG_COLOR: COLORREF = COLORREF(0x3b3b3b);
const ICON_SIZE: i32 = 64;
const WINDOW_BORDER_SIZE: i32 = 10;
const ICON_BORDER_SIZE: i32 = 4;

pub fn start(config: &Config) -> Result<()> {
    info!("start config={:?}", config);
    App::start(config)
}

/// When the taskbar is created, it registers a message with the "TaskbarCreated" string and then broadcasts this message to all top-level windows
/// When the application receives this message, it should assume that any taskbar icons it added have been removed and add them again.
static S_U_TASKBAR_RESTART: Lazy<u32> =
    Lazy::new(|| unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) });

pub struct App {
    hwnd: HWND,
    trayicon: Option<TrayIcon>,
    startup: Startup,
    config: Config,
    switch_windows_state: SwitchWindowsState,
    switch_apps_state: Option<SwtichAppsState>,
    uwp_icons: HashMap<String, Vec<u8>>,
}

impl App {
    pub fn start(config: &Config) -> Result<()> {
        let hwnd = Self::create_window()?;

        let _foreground_watcher = ForegroundWatcher::init(&config.switch_windows_blacklist)?;
        let _keyboard_listener = KeyboardListener::init(hwnd, &config.to_hotkeys())?;

        let trayicon = match config.trayicon {
            true => Some(TrayIcon::create()),
            false => None,
        };

        let startup = Startup::init()?;

        let mut app = App {
            hwnd,
            trayicon,
            startup,
            config: config.clone(),
            switch_windows_state: SwitchWindowsState {
                cache: None,
                modifier_released: true,
            },
            switch_apps_state: None,
            uwp_icons: Default::default(),
        };

        app.set_trayicon()?;

        let app_ptr = Box::into_raw(Box::new(app)) as _;
        check_error(|| set_window_user_data(hwnd, app_ptr))
            .map_err(|err| anyhow!("Failed to set window ptr, {err}"))?;

        Self::eventloop()
    }

    fn eventloop() -> Result<()> {
        let mut message = MSG::default();
        loop {
            let ret = unsafe { GetMessageW(&mut message, HWND(0), 0, 0) };
            match ret.0 {
                -1 => {
                    unsafe { GetLastError() }.ok()?;
                }
                0 => break,
                _ => unsafe {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                },
            }
        }

        Ok(())
    }

    fn create_window() -> Result<HWND> {
        let hinstance = unsafe { GetModuleHandleW(None) }
            .map_err(|err| anyhow!("Failed to get current module handle, {err}"))?;

        let window_class = WNDCLASSW {
            hInstance: hinstance,
            lpszClassName: NAME,
            hbrBackground: unsafe { CreateSolidBrush(BG_COLOR) },
            lpfnWndProc: Some(App::window_proc),
            ..Default::default()
        };

        let atom = unsafe { RegisterClassW(&window_class) }
            .check_error()
            .map_err(|err| anyhow!("Failed to register class, {err}"))?;

        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
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
                None,
            )
        }
        .check_error()
        .map_err(|err| anyhow!("Failed to create windows, {err}"))?;

        // hide caption
        let mut style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32;
        style &= !WS_CAPTION.0;
        unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, style as _) };

        Ok(hwnd)
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
                let app = get_app(hwnd)?;
                if let Some(trayicon) = app.trayicon.as_mut() {
                    let keycode = lparam.0 as u32;
                    if keycode == WM_LBUTTONUP || keycode == WM_RBUTTONUP {
                        trayicon.show(app.startup.is_enable)?;
                    }
                }
                return Ok(LRESULT(0));
            }
            WM_USER_MODIFIER_KEYUP => {
                debug!("message WM_USER_MODIFIER_KEYUP {}", wparam.0);
                let app = get_app(hwnd)?;
                let modifier = wparam.0 as u16;
                if modifier == app.config.switch_windows_hotkey.get_modifier() {
                    app.switch_windows_state.modifier_released = true;
                }
                if modifier == app.config.switch_apps_hotkey.get_modifier() {
                    if let Some(state) = app.switch_apps_state.take() {
                        if let Some((_, id, module_path)) = state.apps.get(state.index) {
                            set_foregound_window(*id, module_path);
                        }
                        for (hicon, _, _) in state.apps {
                            unsafe { DestroyIcon(hicon) };
                        }
                        unsafe { ShowWindow(hwnd, SW_HIDE) };
                    }
                }
            }
            WM_USER_HOOTKEY => {
                debug!("message WM_USER_HOOTKEY {}", wparam.0);
                let app = get_app(hwnd)?;
                let reverse = lparam.0 == 1;
                let hotkey_id = wparam.0 as u32;
                if hotkey_id == app.config.switch_windows_hotkey.id {
                    app.switch_windows(reverse)?;
                } else if hotkey_id == app.config.switch_apps_hotkey.id {
                    app.switch_apps(reverse)?;
                    unsafe {
                        RedrawWindow(hwnd, None, HRGN::default(), RDW_ERASE | RDW_INVALIDATE)
                    };
                }
            }
            WM_COMMAND => {
                let value = wparam.0 as u32;
                let kind = ((value >> 16) & 0xffff) as u16;
                let id = value & 0xffff;
                if kind == 0 {
                    match id {
                        IDM_EXIT => {
                            if let Ok(app) = get_app(hwnd) {
                                unsafe { drop(Box::from_raw(app)) }
                            }
                            unsafe { PostQuitMessage(0) }
                        }
                        IDM_STARTUP => {
                            let app = get_app(hwnd)?;
                            app.startup.toggle()?;
                        }
                        _ => {}
                    }
                }
            }
            WM_PAINT => {
                let app = get_app(hwnd)?;
                app.paint()?;
            }
            _ if msg == *S_U_TASKBAR_RESTART => {
                let app = get_app(hwnd)?;
                app.set_trayicon()?;
            }
            _ => {}
        }
        Ok(unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) })
    }

    pub fn switch_windows(&mut self, reverse: bool) -> Result<bool> {
        debug!("switch windows enter {:?}", self.switch_windows_state);
        let windows = list_windows(false)?;
        let foreground_window = get_foreground_window();
        let foreground_pid = get_window_pid(foreground_window);
        let module_path = match get_module_path(foreground_window, foreground_pid) {
            Some(v) => v,
            None => return Ok(false),
        };
        match windows.get(&module_path) {
            None => Ok(false),
            Some(windows) => {
                debug!("switch windows {module_path} {windows:?}");
                let windows_len = windows.len();
                if windows_len == 1 {
                    return Ok(false);
                }
                let current_id = windows[0];
                let mut index = 1;
                let mut state_id = current_id;
                let mut state_windows = vec![];
                if windows_len > 2 {
                    if let Some((cache_module_path, cache_id, cache_index, cache_windows)) =
                        self.switch_windows_state.cache.as_ref()
                    {
                        if cache_module_path == &module_path {
                            if self.switch_windows_state.modifier_released {
                                if *cache_id != current_id {
                                    if let Some((i, _)) =
                                        windows.iter().enumerate().find(|(_, v)| *v == cache_id)
                                    {
                                        index = i;
                                    }
                                }
                            } else {
                                state_id = *cache_id;
                                let mut windows_set: IndexSet<isize> =
                                    windows.iter().map(|v| v.0).collect();
                                for id in cache_windows {
                                    if windows_set.contains(id) {
                                        state_windows.push(*id);
                                        windows_set.remove(id);
                                    }
                                }
                                state_windows.extend(windows_set);
                                index = if reverse {
                                    if *cache_index == 0 {
                                        windows_len - 1
                                    } else {
                                        cache_index - 1
                                    }
                                } else if *cache_index >= windows_len - 1 {
                                    0
                                } else {
                                    cache_index + 1
                                };
                            }
                        }
                    }
                }
                if state_windows.is_empty() {
                    state_windows = windows.iter().map(|v| v.0).collect();
                }
                let hwnd = HWND(state_windows[index]);
                self.switch_windows_state = SwitchWindowsState {
                    cache: Some((module_path.clone(), state_id, index, state_windows)),
                    modifier_released: false,
                };
                debug!(
                    "switch windows done {:?} {:?}",
                    hwnd, self.switch_windows_state
                );
                set_foregound_window(hwnd, &module_path);

                Ok(true)
            }
        }
    }

    fn switch_apps(&mut self, reverse: bool) -> Result<()> {
        debug!("switch apps enter {:?}", self.switch_apps_state);
        if let Some(state) = self.switch_apps_state.as_mut() {
            if reverse {
                if state.index == 0 {
                    state.index = state.apps.len() - 1;
                } else {
                    state.index -= 1;
                }
            } else if state.index == state.apps.len() - 1 {
                state.index = 0;
            } else {
                state.index += 1;
            };
            return Ok(());
        }
        let hwnd = self.hwnd;
        let windows = list_windows(true)?;
        let mut apps = vec![];
        for (module_path, hwnds) in windows.iter() {
            let module_hwnd = hwnds[0];
            let mut module_hicon = None;
            if module_path.starts_with("C:\\Program Files\\WindowsApps") {
                if let Some(data) = self.uwp_icons.get(module_path) {
                    module_hicon = create_hicon_from_resource(data)
                } else if let Some(data) = get_uwp_icon_data(module_path) {
                    module_hicon = create_hicon_from_resource(&data);
                    self.uwp_icons.insert(module_path.clone(), data);
                }
            }
            if module_hicon.is_none() {
                module_hicon = get_module_icon(module_hwnd);
            }
            if let Some(hicon) = module_hicon {
                apps.push((hicon, module_hwnd, module_path.clone()));
            }
        }
        let num_apps = apps.len() as i32;
        if num_apps == 0 {
            return Ok(());
        }
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..MONITORINFO::default()
        };
        unsafe {
            let mut cursor = POINT::default();
            GetCursorPos(&mut cursor);

            let hmonitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
            GetMonitorInfoW(hmonitor, &mut mi);
        }

        let monitor_rect = mi.rcMonitor;
        let monitor_width = monitor_rect.right - monitor_rect.left;
        let monitor_height = monitor_rect.bottom - monitor_rect.top;
        let icon_size = ((monitor_width - 2 * WINDOW_BORDER_SIZE) / num_apps
            - ICON_BORDER_SIZE * 2)
            .min(ICON_SIZE);
        let item_size = icon_size + ICON_BORDER_SIZE * 2;
        let window_width = item_size * num_apps + WINDOW_BORDER_SIZE * 2;
        let window_height = item_size + WINDOW_BORDER_SIZE * 2;

        // Calculate the position to center the window
        let x = monitor_rect.left + (monitor_width - window_width) / 2;
        let y = monitor_rect.top + (monitor_height - window_height) / 2;

        unsafe {
            // Change busy cursor to array cursor
            if let Ok(hcursor) = LoadCursorW(HMODULE(0), IDC_ARROW) {
                SetCursor(hcursor);
            }
            SetFocus(hwnd);
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                x,
                y,
                window_width,
                window_height,
                SWP_SHOWWINDOW,
            );
        }

        let index = if apps.len() == 1 { 0 } else { 1 };

        self.switch_apps_state = Some(SwtichAppsState {
            apps,
            index,
            icon_size,
        });
        debug!("switch apps done {:?}", self.switch_apps_state);
        Ok(())
    }

    fn paint(&mut self) -> Result<()> {
        unsafe {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(self.hwnd, &mut ps);
            if let Some(state) = self.switch_apps_state.as_ref() {
                let cy = WINDOW_BORDER_SIZE + ICON_BORDER_SIZE;
                let item_size = state.icon_size + 2 * ICON_BORDER_SIZE;
                for (i, (hicon, _, _)) in state.apps.iter().enumerate() {
                    let brush = if i == state.index {
                        CreateSolidBrush(FG_COLOR)
                    } else {
                        CreateSolidBrush(BG_COLOR)
                    };
                    if i == state.index {
                        let left = WINDOW_BORDER_SIZE + item_size * (i as i32);
                        let top = WINDOW_BORDER_SIZE;
                        let right = left + item_size;
                        let bottom = top + item_size;
                        let rect = RECT {
                            left,
                            top,
                            right,
                            bottom,
                        };
                        FillRect(hdc, &rect as _, CreateSolidBrush(FG_COLOR));
                    }
                    let cx = WINDOW_BORDER_SIZE + item_size * (i as i32) + ICON_BORDER_SIZE;
                    DrawIconEx(
                        hdc,
                        cx,
                        cy,
                        *hicon,
                        state.icon_size,
                        state.icon_size,
                        0,
                        brush,
                        DI_NORMAL,
                    );
                }
            }
            EndPaint(self.hwnd, &ps);
        }

        Ok(())
    }
}

fn get_app(hwnd: HWND) -> Result<&'static mut App> {
    unsafe {
        let ptr = check_error(|| get_window_user_data(hwnd))
            .map_err(|err| anyhow!("Failed to get window ptr, {err}"))?;
        let tx: &mut App = &mut *(ptr as *mut _);
        Ok(tx)
    }
}

#[derive(Debug)]
struct SwitchWindowsState {
    cache: Option<(String, HWND, usize, Vec<isize>)>,
    modifier_released: bool,
}

#[derive(Debug)]
struct SwtichAppsState {
    apps: Vec<(HICON, HWND, String)>,
    index: usize,
    icon_size: i32,
}
