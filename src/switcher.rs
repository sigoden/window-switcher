use crate::debug;
use crate::utils::{
    get_foreground_window, get_module_path, get_window_pid, is_iconic, is_popup_window,
    is_special_window, is_window_cloaked, is_window_visible, switch_to,
};
use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::EnumWindows;

pub struct Switcher {
    switch_windows_state: Option<SwitcherState>,
}

impl Switcher {
    pub fn new() -> Self {
        Self {
            switch_windows_state: None,
        }
    }

    pub fn next_window(&mut self, back: bool) -> Result<bool> {
        let windows = Self::get_windows(false)?;
        let foreground_window = get_foreground_window();
        let foreground_pid = get_window_pid(foreground_window);
        let module_path = get_module_path(foreground_pid);
        if module_path.is_empty() {
            return Ok(false);
        }
        match windows.get(&module_path) {
            None => Ok(false),
            Some(windows) => {
                debug!("switch windows {:?}", windows);
                let len = windows.len();
                if len == 1 {
                    return Ok(false);
                }
                let current_id = windows[0];
                let mut index = 1;
                let next_window_id = windows[1];
                let mut new_state_id = next_window_id;
                if len > 2 {
                    if let Some(state) = self.switch_windows_state.as_ref() {
                        debug!("{state:?}");
                        if state.path == module_path {
                            if back {
                                if state.id != current_id {
                                    if let Some((i, _)) =
                                        windows.iter().enumerate().find(|(_, v)| **v == state.id)
                                    {
                                        if index == 1 {
                                            new_state_id = current_id;
                                        }
                                        index = i;
                                    }
                                }
                            } else {
                                index = (state.index + 1).min(windows.len() - 1);
                            }
                        }
                    }
                }
                self.switch_windows_state = Some(SwitcherState {
                    path: module_path,
                    index,
                    id: new_state_id,
                });
                let hwnd = HWND(windows[index]);
                debug!("{:?} {:?}", hwnd, self.switch_windows_state);
                switch_to(hwnd)?;

                Ok(true)
            }
        }
    }

    fn get_windows(no_minimal: bool) -> Result<IndexMap<String, Vec<isize>>> {
        let mut data = EnumWindowsData {
            no_minimal,
            windows: Default::default(),
        };
        unsafe { EnumWindows(Some(enum_window), LPARAM(&mut data as *mut _ as isize)).ok() }
            .map_err(|e| anyhow!("Fail to get windows {}", e))?;
        Ok(data.windows)
    }
}

#[derive(Debug)]
struct SwitcherState {
    path: String,
    id: isize,
    index: usize,
}

#[derive(Debug)]
struct EnumWindowsData {
    no_minimal: bool,
    windows: IndexMap<String, Vec<isize>>,
}

extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let state: &mut EnumWindowsData = unsafe { &mut *(lparam.0 as *mut _) };
    if state.no_minimal && is_iconic(hwnd) {
        return BOOL(1);
    }
    if !is_window_visible(hwnd)
        || is_window_cloaked(hwnd)
        || is_popup_window(hwnd)
        || is_special_window(hwnd)
    {
        return BOOL(1);
    }
    let pid = get_window_pid(hwnd);
    let module_path = get_module_path(pid);
    state.windows.entry(module_path).or_default().push(hwnd.0);
    BOOL(1)
}
