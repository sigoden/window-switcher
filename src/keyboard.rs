use crate::{
    app::{WM_USER_HOOTKEY, WM_USER_MODIFIER_KEYUP},
    config::{Hotkey, SWITCH_WINDOWS_HOTKEY_ID},
    foregound::IS_FOREGROUND_IN_BLACKLIST,
};

use anyhow::{anyhow, Result};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SendMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT,
    WH_KEYBOARD_LL,
};

static mut KEYBOARD_STATE: Vec<HotKeyState> = vec![];
static mut WINDOW: HWND = HWND(0);

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
        unsafe {
            KEYBOARD_STATE = hotkeys
                .iter()
                .map(|hotkey| HotKeyState {
                    hotkey: (*hotkey).clone(),
                    is_modifier_pressed: false,
                    count_code_key_pressed: 0,
                })
                .collect()
        }

        Ok(Self { hook })
    }
}

impl Drop for KeyboardListener {
    fn drop(&mut self) {
        debug!("keyboard listener destoryed");
        if !self.hook.is_invalid() {
            unsafe { UnhookWindowsHookEx(self.hook) };
        }
    }
}

#[derive(Debug)]
struct HotKeyState {
    hotkey: Hotkey,
    is_modifier_pressed: bool,
    count_code_key_pressed: u32,
}

unsafe extern "system" fn keyboard_proc(code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    let kbd_data: &KBDLLHOOKSTRUCT = &*(l_param.0 as *const _);
    let vk_code = VIRTUAL_KEY(kbd_data.vkCode as _);
    let mut is_modifier = false;
    let is_key_pressed = || kbd_data.flags.0 & 128 == 0;
    for state in KEYBOARD_STATE.iter_mut() {
        if vk_code == state.hotkey.modifier {
            is_modifier = true;
            if is_key_pressed() {
                state.is_modifier_pressed = true;
            } else {
                state.is_modifier_pressed = false;
                unsafe {
                    SendMessageW(
                        WINDOW,
                        WM_USER_MODIFIER_KEYUP,
                        WPARAM(vk_code.0 as _),
                        LPARAM(0),
                    )
                };
            }
            state.count_code_key_pressed = 0;
        }
    }
    if !is_modifier {
        for state in KEYBOARD_STATE.iter_mut() {
            if vk_code.0 == state.hotkey.code && is_key_pressed() && state.is_modifier_pressed {
                let count = state.count_code_key_pressed;
                state.count_code_key_pressed += 1;
                let id = state.hotkey.id;
                if id != SWITCH_WINDOWS_HOTKEY_ID || !IS_FOREGROUND_IN_BLACKLIST {
                    unsafe {
                        SendMessageW(WINDOW, WM_USER_HOOTKEY, WPARAM(id as _), LPARAM(count as _))
                    };
                    return LRESULT(1);
                }
            }
        }
    }
    CallNextHookEx(None, code, w_param, l_param)
}
