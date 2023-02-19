use crate::utils::{
    get_foreground_window, get_module_path, get_window_pid, is_iconic, is_popup_window,
    is_special_window, is_window_cloaked, is_window_fixed_top, is_window_visible, switch_to,
};
use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::EnumWindows;

pub struct Switcher {
    switch_windows_state: Option<SwitcherState>,
    switch_apps_state: Option<SwitcherState>,
}

impl Switcher {
    pub fn new() -> Self {
        Self {
            switch_windows_state: None,
            switch_apps_state: None,
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
                let windows_len = windows.len();
                if windows_len == 1 {
                    return Ok(false);
                }
                let current_id = windows[0];
                let mut index = 1;
                let mut state_id = current_id;
                if windows_len > 2 {
                    if let Some(state) = self.switch_windows_state.as_ref() {
                        debug!("{state:?}");
                        if state.path == module_path {
                            if back {
                                if state.id != current_id {
                                    if let Some((i, _)) =
                                        windows.iter().enumerate().find(|(_, v)| **v == state.id)
                                    {
                                        index = i;
                                    }
                                }
                            } else {
                                index = (state.index + 1).min(windows_len - 1);
                                state_id = state.id;
                            }
                        }
                    }
                }
                self.switch_windows_state = Some(SwitcherState {
                    path: module_path,
                    index,
                    id: state_id,
                });
                let hwnd = HWND(windows[index]);
                debug!("{:?} {:?}", hwnd, self.switch_windows_state);
                switch_to(hwnd)?;

                Ok(true)
            }
        }
    }

    pub fn next_app(&mut self, back: bool) -> Result<bool> {
        let windows = Self::get_windows(true)?;
        let module_paths: Vec<&String> = windows.keys().collect();
        debug!("switch apps {:?}", module_paths);
        let module_paths_len = module_paths.len();
        if module_paths_len == 1 {
            return Ok(false);
        }
        let current_path = module_paths[0];
        let mut index = 1;
        let mut state_path = current_path;
        if module_paths_len > 2 {
            if let Some(state) = self.switch_apps_state.as_ref() {
                debug!("{state:?}");
                if back {
                    if &state.path != current_path {
                        if let Some((i, _)) = module_paths
                            .iter()
                            .enumerate()
                            .find(|(_, v)| **v == &state.path)
                        {
                            index = i;
                        }
                    }
                } else {
                    index = (state.index + 1).min(module_paths.len() - 1);
                    state_path = &state.path;
                }
            }
        }

        self.switch_apps_state = Some(SwitcherState {
            path: state_path.to_string(),
            index,
            id: 0,
        });
        let hwnd = HWND(windows[module_paths[index]][0]);
        switch_to(hwnd)?;

        Ok(true)
    }

    fn get_windows(is_switch_apps: bool) -> Result<IndexMap<String, Vec<isize>>> {
        let mut data = EnumWindowsData {
            is_switch_apps,
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
    is_switch_apps: bool,
    windows: IndexMap<String, Vec<isize>>,
}

extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let state: &mut EnumWindowsData = unsafe { &mut *(lparam.0 as *mut _) };
    if state.is_switch_apps && (is_iconic(hwnd) || is_window_fixed_top(hwnd)) {
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
