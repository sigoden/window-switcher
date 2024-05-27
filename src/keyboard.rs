use crate::{
    app::{
        WM_USER_SWITCH_APPS, WM_USER_SWITCH_APPS_CANCEL, WM_USER_SWITCH_APPS_DONE,
        WM_USER_SWITCH_WINDOWS, WM_USER_SWITCH_WINDOWS_DONE,
    },
    config::{Hotkey, SWITCH_APPS_HOTKEY_ID, SWITCH_WINDOWS_HOTKEY_ID},
    foreground::IS_FOREGROUND_IN_BLACKLIST,
};

use anyhow::{anyhow, Result};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::{
    Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_ESCAPE, VK_LSHIFT, VK_RSHIFT},
    WindowsAndMessaging::{
        CallNextHookEx, SendMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
        KBDLLHOOKSTRUCT, WH_KEYBOARD_LL,
    },
};

static mut KEYBOARD_STATE: Vec<HotKeyState> = vec![];
static mut WINDOW: HWND = HWND(0);
static mut IS_SHIFT_PRESSED: bool = false;
static mut PREVIOUS_KEYCODE: u16 = 0;

#[derive(Debug)]
pub struct KeyboardListener {
    hook: HHOOK,
}

impl KeyboardListener {
    pub fn init(hwnd: HWND, hotkeys: &[&Hotkey]) -> Result<Self> {
        unsafe { WINDOW = hwnd }
        let hook = unsafe {
            let hinstance = { GetModuleHandleW(None) }
                .map_err(|err| anyhow!("Failed to get module handle, {err}"))?;
            SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), hinstance, 0)
        }
        .map_err(|err| anyhow!("Failed to set windows hook, {err}"))?;
        info!("keyboard listener start");
        unsafe {
            KEYBOARD_STATE = hotkeys
                .iter()
                .map(|hotkey| HotKeyState {
                    hotkey: (*hotkey).clone(),
                    is_modifier_pressed: false,
                })
                .collect()
        }

        Ok(Self { hook })
    }
}

impl Drop for KeyboardListener {
    fn drop(&mut self) {
        debug!("keyboard listener destroyed");
        if !self.hook.is_invalid() {
            let _ = unsafe { UnhookWindowsHookEx(self.hook) };
        }
    }
}

#[derive(Debug)]
struct HotKeyState {
    hotkey: Hotkey,
    is_modifier_pressed: bool,
}

unsafe extern "system" fn keyboard_proc(code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    let kbd_data: &KBDLLHOOKSTRUCT = &*(l_param.0 as *const _);
    debug!("keyboard {kbd_data:?}");
    let vk_code = VIRTUAL_KEY(kbd_data.vkCode as _);
    let mut is_modifier = false;
    let is_key_pressed = || kbd_data.flags.0 & 128 == 0;
    if [VK_LSHIFT, VK_RSHIFT].contains(&vk_code) {
        IS_SHIFT_PRESSED = is_key_pressed();
    }
    for state in KEYBOARD_STATE.iter_mut() {
        if state.hotkey.modifier.contains(&vk_code) {
            is_modifier = true;
            if is_key_pressed() {
                state.is_modifier_pressed = true;
            } else {
                state.is_modifier_pressed = false;
                if PREVIOUS_KEYCODE == state.hotkey.code {
                    let id = state.hotkey.id;
                    if id == SWITCH_APPS_HOTKEY_ID {
                        unsafe {
                            SendMessageW(WINDOW, WM_USER_SWITCH_APPS_DONE, WPARAM(0), LPARAM(0))
                        };
                    } else if id == SWITCH_WINDOWS_HOTKEY_ID {
                        unsafe {
                            SendMessageW(WINDOW, WM_USER_SWITCH_WINDOWS_DONE, WPARAM(0), LPARAM(0))
                        };
                    }
                }
            }
        }
    }
    if !is_modifier {
        for state in KEYBOARD_STATE.iter_mut() {
            if is_key_pressed() && state.is_modifier_pressed {
                let id = state.hotkey.id;
                if vk_code.0 == state.hotkey.code {
                    let reverse = if IS_SHIFT_PRESSED { 1 } else { 0 };
                    if id == SWITCH_APPS_HOTKEY_ID {
                        unsafe {
                            SendMessageW(WINDOW, WM_USER_SWITCH_APPS, WPARAM(0), LPARAM(reverse))
                        };
                        PREVIOUS_KEYCODE = vk_code.0;
                        return LRESULT(1);
                    } else if id == SWITCH_WINDOWS_HOTKEY_ID && !IS_FOREGROUND_IN_BLACKLIST {
                        unsafe {
                            SendMessageW(WINDOW, WM_USER_SWITCH_WINDOWS, WPARAM(0), LPARAM(reverse))
                        };
                        PREVIOUS_KEYCODE = vk_code.0;
                        return LRESULT(1);
                    }
                } else if vk_code == VK_ESCAPE && id == SWITCH_APPS_HOTKEY_ID {
                    unsafe {
                        SendMessageW(WINDOW, WM_USER_SWITCH_APPS_CANCEL, WPARAM(0), LPARAM(0))
                    };
                    PREVIOUS_KEYCODE = vk_code.0;
                    return LRESULT(1);
                }
            }
        }
    }
    CallNextHookEx(None, code, w_param, l_param)
}
