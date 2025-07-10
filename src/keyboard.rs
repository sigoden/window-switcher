use crate::{
    app::{
        WM_USER_SWITCH_APPS, WM_USER_SWITCH_APPS_CANCEL, WM_USER_SWITCH_APPS_DONE,
        WM_USER_SWITCH_WINDOWS, WM_USER_SWITCH_WINDOWS_DONE,
    },
    config::{Hotkey, SWITCH_APPS_HOTKEY_ID, SWITCH_WINDOWS_HOTKEY_ID},
    foreground::IS_FOREGROUND_IN_BLACKLIST,
};

use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use std::sync::LazyLock;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Input::KeyboardAndMouse::{SCANCODE_LSHIFT, SCANCODE_RSHIFT},
        WindowsAndMessaging::{
            CallNextHookEx, SendMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
            KBDLLHOOKSTRUCT, LLKHF_UP, WH_KEYBOARD_LL,
        },
    },
};

static KEYBOARD_STATE: LazyLock<Mutex<Vec<HotKeyState>>> = LazyLock::new(|| Mutex::new(Vec::new()));
static mut WINDOW: HWND = HWND(0 as _);
static mut IS_SHIFT_PRESSED: bool = false;
static mut PREVIOUS_KEYCODE: u32 = 0;

#[derive(Debug)]
pub struct KeyboardListener {
    hook: HHOOK,
}

impl KeyboardListener {
    pub fn init(hwnd: HWND, hotkeys: &[&Hotkey]) -> Result<Self> {
        unsafe { WINDOW = hwnd }

        let keyboard_state = hotkeys
            .iter()
            .map(|hotkey| HotKeyState {
                hotkey: (*hotkey).clone(),
                is_modifier_pressed: false,
            })
            .collect();
        *KEYBOARD_STATE.lock() = keyboard_state;

        let hook = unsafe {
            let hinstance = { GetModuleHandleW(None) }
                .map_err(|err| anyhow!("Failed to get module handle, {err}"))?;
            SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_proc),
                Some(hinstance.into()),
                0,
            )
        }
        .map_err(|err| anyhow!("Failed to set windows hook, {err}"))?;
        info!("keyboard listener start");

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
    let mut is_modifier = false;
    let scan_code = kbd_data.scanCode;
    let is_key_pressed = || kbd_data.flags.0 & LLKHF_UP.0 == 0;
    if [SCANCODE_LSHIFT, SCANCODE_RSHIFT].contains(&scan_code) {
        IS_SHIFT_PRESSED = is_key_pressed();
    }
    for state in KEYBOARD_STATE.lock().iter_mut() {
        if state.hotkey.modifier.contains(&scan_code) {
            is_modifier = true;
            if is_key_pressed() {
                state.is_modifier_pressed = true;
            } else {
                state.is_modifier_pressed = false;
                if PREVIOUS_KEYCODE == state.hotkey.code {
                    let id = state.hotkey.id;
                    if id == SWITCH_APPS_HOTKEY_ID {
                        unsafe { SendMessageW(WINDOW, WM_USER_SWITCH_APPS_DONE, None, None) };
                    } else if id == SWITCH_WINDOWS_HOTKEY_ID {
                        unsafe { SendMessageW(WINDOW, WM_USER_SWITCH_WINDOWS_DONE, None, None) };
                    }
                }
            }
        }
    }
    if !is_modifier {
        for state in KEYBOARD_STATE.lock().iter_mut() {
            if is_key_pressed() && state.is_modifier_pressed {
                let id = state.hotkey.id;
                if scan_code == state.hotkey.code {
                    let reverse = if IS_SHIFT_PRESSED { 1 } else { 0 };
                    if id == SWITCH_APPS_HOTKEY_ID {
                        unsafe {
                            SendMessageW(WINDOW, WM_USER_SWITCH_APPS, None, Some(LPARAM(reverse)))
                        };
                        PREVIOUS_KEYCODE = scan_code;
                        return LRESULT(1);
                    } else if id == SWITCH_WINDOWS_HOTKEY_ID && !IS_FOREGROUND_IN_BLACKLIST {
                        unsafe {
                            SendMessageW(
                                WINDOW,
                                WM_USER_SWITCH_WINDOWS,
                                None,
                                Some(LPARAM(reverse)),
                            )
                        };
                        PREVIOUS_KEYCODE = scan_code;
                        return LRESULT(1);
                    }
                } else if scan_code == 0x01 && id == SWITCH_APPS_HOTKEY_ID {
                    unsafe { SendMessageW(WINDOW, WM_USER_SWITCH_APPS_CANCEL, None, None) };
                    PREVIOUS_KEYCODE = scan_code;
                    return LRESULT(1);
                }
            }
        }
    }
    CallNextHookEx(None, code, w_param, l_param)
}
