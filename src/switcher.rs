use crate::utils::{
    get_foreground_window, get_window_module_path, get_window_title, is_window_minimized,
    is_window_visible, switch_to,
};
use crate::{log_error, log_info};
use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use std::ops::Deref;
use windows::core::GUID;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows::Win32::UI::Shell::IVirtualDesktopManager;
use windows::Win32::UI::WindowsAndMessaging::EnumWindows;

#[allow(non_upper_case_globals)]
const CLSID_VirtualDesktopManager: GUID = GUID::from_u128(0xaa509086_5ca9_4c25_8f95_589d3c07b48a);

pub struct Switcher {
    windows: IndexMap<String, Vec<isize>>,
    virtual_desktop: Option<VirtualDesktop>,
    is_apps_switch: bool,
    state: Option<SwitcherState>,
    state2: Option<SwitcherState>,
}

impl Switcher {
    pub fn new(virtual_desktop: Option<VirtualDesktop>) -> Self {
        Self {
            windows: IndexMap::new(),
            virtual_desktop,
            is_apps_switch: false,
            state: None,
            state2: None,
        }
    }

    pub fn switch_window(&mut self, back: bool) -> Result<bool> {
        self.is_apps_switch = false;
        self.enum_windows()?;

        let current_window = get_foreground_window();
        let module_path = get_window_module_path(current_window);
        if module_path.is_empty() {
            self.windows.clear();
            return Ok(false);
        }
        match self.windows.get(&module_path) {
            None => Ok(false),
            Some(windows) => {
                log_info!("switch windows {:?}", windows);
                let len = windows.len();
                if len == 1 {
                    return Ok(false);
                }
                let current_id = windows[0];
                let mut index = 1;
                let mut new_state_id = windows[index];
                if len > 2 {
                    if let Some(state) = self.state.as_ref() {
                        log_info!("switch windows state {:?}", state);
                        if state.path == module_path {
                            if back {
                                if state.id != current_id {
                                    if let Some((i, _)) =
                                        windows.iter().enumerate().find(|(_, v)| **v == state.id)
                                    {
                                        index = i
                                    }
                                }
                                new_state_id = windows[index]
                            } else {
                                index = (state.index + 1).min(windows.len() - 1);
                            }
                        }
                    }
                }
                self.state = Some(SwitcherState {
                    path: module_path,
                    index,
                    id: new_state_id,
                });
                let hwnd = HWND(windows[index]);
                switch_to(hwnd)?;

                self.windows.clear();

                Ok(true)
            }
        }
    }

    pub fn switch_app(&mut self, back: bool) -> Result<bool> {
        self.is_apps_switch = true;
        self.enum_windows()?;
        let mut index = 1;
        let module_paths: Vec<&String> = self.windows.keys().collect();
        let mut new_state_path = module_paths[0];
        log_info!("switch apps {:?}", module_paths);
        if module_paths.len() == 1 {
            self.windows.clear();
            return Ok(false);
        }
        if module_paths.len() > 2 {
            if let Some(state) = self.state2.as_ref() {
                log_info!("switch apps state {:?}", state);
                if back {
                    if &state.path != module_paths[0] {
                        if let Some((i, path)) = module_paths
                            .iter()
                            .enumerate()
                            .find(|(_, v)| **v == &state.path)
                        {
                            if *path != module_paths[1] {
                                new_state_path = path;
                            }
                            index = i;
                        }
                    }
                } else {
                    index = (state.index + 1).min(module_paths.len() - 1);
                    if &state.path != module_paths[index] {
                        new_state_path = &state.path;
                    }
                }
            }
        }

        self.state2 = Some(SwitcherState {
            path: new_state_path.to_string(),
            index,
            id: 0,
        });
        let hwnd = HWND(self.windows[module_paths[index]][0]);
        switch_to(hwnd)?;

        self.windows.clear();
        Ok(true)
    }

    fn enum_windows(&mut self) -> Result<()> {
        unsafe { EnumWindows(Some(enum_window), LPARAM(self as *mut _ as isize)).ok() }
            .map_err(|e| anyhow!("Fail to enum windows {}", e))
    }

    fn is_window_on_desktop(&self, hwnd: HWND) -> bool {
        if let Some(virtual_desktop) = self.virtual_desktop.as_ref() {
            match virtual_desktop.is_window_on_current_virtual_desktop(hwnd) {
                Ok(on) => {
                    if !on {
                        return false;
                    }
                }
                Err(err) => {
                    log_error!(err.to_string());
                }
            }
        }
        true
    }
}

#[derive(Debug)]
struct SwitcherState {
    path: String,
    id: isize,
    index: usize,
}

#[derive(Clone)]
pub struct VirtualDesktop {
    inner: IVirtualDesktopManager,
}

impl VirtualDesktop {
    pub fn create() -> Result<Self> {
        let inner = unsafe {
            CoCreateInstance(&CLSID_VirtualDesktopManager, None, CLSCTX_ALL)
                .map_err(|e| anyhow!("Fail to access virtual desktop com, {}", e))?
        };
        Ok(Self { inner })
    }
    pub fn is_window_on_current_virtual_desktop(&self, hwnd: HWND) -> Result<bool> {
        let ret = unsafe { self.inner.IsWindowOnCurrentVirtualDesktop(hwnd) }
            .map_err(|e| anyhow!("Fail to check current desktop, {}", e))?;
        Ok(ret.as_bool())
    }
}

impl Deref for VirtualDesktop {
    type Target = IVirtualDesktopManager;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let switcher: &mut Switcher = unsafe { &mut *(lparam.0 as *mut Switcher) };

    let ok: BOOL = true.into();

    if !switcher.is_window_on_desktop(hwnd) {
        return ok;
    }

    if !is_window_visible(hwnd) {
        return ok;
    }

    if switcher.is_apps_switch && is_window_minimized(hwnd) {
        return ok;
    }

    let title = get_window_title(hwnd);
    if title.is_empty() {
        return ok;
    }

    let module_path = get_window_module_path(hwnd);
    if module_path.is_empty() {
        return ok;
    }

    if (title == "Program Manager" && module_path.ends_with("explorer.exe"))
        || module_path.ends_with("TextInputHost.exe")
    {
        return ok;
    }
    // log_info!("{:?} {} {}", hwnd, &title, &module_path);
    switcher
        .windows
        .entry(module_path)
        .or_default()
        .push(hwnd.0);

    true.into()
}
