use crate::{
    app::{
        WM_USER_SWITCH_APPS, WM_USER_SWITCH_APPS_CANCEL, WM_USER_SWITCH_APPS_DONE,
        WM_USER_SWITCH_WINDOWS, WM_USER_SWITCH_WINDOWS_CANCEL, WM_USER_SWITCH_WINDOWS_DONE,
    },
    config::{Hotkey, SWITCH_APPS_HOTKEY_ID, SWITCH_WINDOWS_HOTKEY_ID},
    foreground::IS_FOREGROUND_IN_BLACKLIST,
};

use anyhow::{anyhow, Result};
use indexmap::IndexSet;
use parking_lot::Mutex;
use std::sync::LazyLock;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Input::KeyboardAndMouse::{SCANCODE_LSHIFT, SCANCODE_RSHIFT},
        WindowsAndMessaging::{
            CallNextHookEx, PostMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
            KBDLLHOOKSTRUCT, LLKHF_UP, WH_KEYBOARD_LL,
        },
    },
};

static KEYBOARD_STATE: LazyLock<Mutex<Vec<HotKeyState>>> = LazyLock::new(|| Mutex::new(Vec::new()));
static mut WINDOW: HWND = HWND(0 as _);
static mut IS_SHIFT_PRESSED: bool = false;
static mut IS_SWITCHING_APPS: bool = false;
static mut IS_SWITCHING_WINDOWS: bool = false;
static mut PREVIOUS_KEYCODE: u32 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SwitchKeyAction {
    Advance { id: u32, reverse: bool },
    Cancel { id: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QueuedSwitchMessage {
    message: u32,
    wparam: usize,
    lparam: isize,
}

fn switch_key_action(
    id: u32,
    configured_code: u32,
    scan_code: u32,
    shift_pressed: bool,
    switching: bool,
) -> Option<SwitchKeyAction> {
    if scan_code == 0x01 && switching {
        Some(SwitchKeyAction::Cancel { id })
    } else if scan_code == configured_code {
        Some(SwitchKeyAction::Advance {
            id,
            reverse: shift_pressed,
        })
    } else {
        None
    }
}

fn message_for_done(id: u32) -> Option<QueuedSwitchMessage> {
    let message = match id {
        SWITCH_APPS_HOTKEY_ID => WM_USER_SWITCH_APPS_DONE,
        SWITCH_WINDOWS_HOTKEY_ID => WM_USER_SWITCH_WINDOWS_DONE,
        _ => return None,
    };
    Some(QueuedSwitchMessage {
        message,
        wparam: 0,
        lparam: 0,
    })
}

fn message_for_action(action: SwitchKeyAction) -> Option<QueuedSwitchMessage> {
    let (message, lparam) = match action {
        SwitchKeyAction::Advance { id, reverse } if id == SWITCH_APPS_HOTKEY_ID => {
            (WM_USER_SWITCH_APPS, reverse as isize)
        }
        SwitchKeyAction::Advance { id, reverse } if id == SWITCH_WINDOWS_HOTKEY_ID => {
            (WM_USER_SWITCH_WINDOWS, reverse as isize)
        }
        SwitchKeyAction::Cancel { id } if id == SWITCH_APPS_HOTKEY_ID => {
            (WM_USER_SWITCH_APPS_CANCEL, 0)
        }
        SwitchKeyAction::Cancel { id } if id == SWITCH_WINDOWS_HOTKEY_ID => {
            (WM_USER_SWITCH_WINDOWS_CANCEL, 0)
        }
        _ => return None,
    };
    Some(QueuedSwitchMessage {
        message,
        wparam: 0,
        lparam,
    })
}

fn dispatch_messages(
    messages: &[QueuedSwitchMessage],
    mut dispatch: impl FnMut(QueuedSwitchMessage),
) {
    for message in messages {
        dispatch(*message);
    }
}

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

fn post_messages(hwnd: HWND, messages: &[QueuedSwitchMessage]) {
    dispatch_messages(messages, |message| unsafe {
        let _ = PostMessageW(
            Some(hwnd),
            message.message,
            WPARAM(message.wparam),
            LPARAM(message.lparam),
        );
    });
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
    let mut keyboard_state = KEYBOARD_STATE.lock();
    let mut send_done_hotkeys: IndexSet<u32> = IndexSet::new();
    let mut send_action_message: Option<SwitchKeyAction> = None;

    for state in keyboard_state.iter_mut() {
        if state.hotkey.modifier.contains(&scan_code) {
            is_modifier = true;
            if is_key_pressed() {
                state.is_modifier_pressed = true;
            } else {
                state.is_modifier_pressed = false;
                if PREVIOUS_KEYCODE == state.hotkey.code {
                    send_done_hotkeys.insert(state.hotkey.id);
                }
            }
        }
    }
    if !is_modifier {
        for state in keyboard_state.iter_mut() {
            if is_key_pressed() && state.is_modifier_pressed {
                let id = state.hotkey.id;
                let switching = if id == SWITCH_APPS_HOTKEY_ID {
                    IS_SWITCHING_APPS
                } else {
                    IS_SWITCHING_WINDOWS
                };
                if let Some(action) = switch_key_action(
                    id,
                    state.hotkey.code,
                    scan_code,
                    IS_SHIFT_PRESSED,
                    switching,
                ) {
                    match action {
                        SwitchKeyAction::Advance { .. }
                            if id == SWITCH_APPS_HOTKEY_ID
                                || (id == SWITCH_WINDOWS_HOTKEY_ID
                                    && !IS_FOREGROUND_IN_BLACKLIST) =>
                        {
                            send_action_message = Some(action);
                            PREVIOUS_KEYCODE = scan_code;
                            break;
                        }
                        SwitchKeyAction::Cancel { .. } => {
                            send_action_message = Some(action);
                            PREVIOUS_KEYCODE = scan_code;
                            break;
                        }
                        SwitchKeyAction::Advance { .. } => {}
                    }
                } else if id == SWITCH_APPS_HOTKEY_ID
                    && [0x48, 0x4b, 0x4d, 0x50].contains(&scan_code)
                    && IS_SWITCHING_APPS
                {
                    // arrow keys
                    let reverse = scan_code == 0x48 || scan_code == 0x4b;
                    send_action_message = Some(SwitchKeyAction::Advance { id, reverse });
                    break;
                }
            }
        }
    }
    drop(keyboard_state);

    let mut queued_messages = Vec::with_capacity(send_done_hotkeys.len() + 1);
    for id in send_done_hotkeys {
        if id == SWITCH_APPS_HOTKEY_ID {
            if let Some(message) = message_for_done(id) {
                queued_messages.push(message);
            }
            IS_SWITCHING_APPS = false;
        } else if id == SWITCH_WINDOWS_HOTKEY_ID {
            if let Some(message) = message_for_done(id) {
                queued_messages.push(message);
            }
            IS_SWITCHING_WINDOWS = false;
        }
    }

    let mut should_swallow = false;
    if let Some(action) = send_action_message {
        match action {
            SwitchKeyAction::Advance { id, .. } if id == SWITCH_APPS_HOTKEY_ID => {
                if let Some(message) = message_for_action(action) {
                    queued_messages.push(message);
                }
                IS_SWITCHING_APPS = true;
                IS_SWITCHING_WINDOWS = false;
                should_swallow = true;
            }
            SwitchKeyAction::Cancel { id } if id == SWITCH_APPS_HOTKEY_ID => {
                if let Some(message) = message_for_action(action) {
                    queued_messages.push(message);
                }
                IS_SWITCHING_APPS = false;
                should_swallow = true;
            }
            SwitchKeyAction::Advance { id, .. } if id == SWITCH_WINDOWS_HOTKEY_ID => {
                if let Some(message) = message_for_action(action) {
                    queued_messages.push(message);
                }
                IS_SWITCHING_APPS = false;
                IS_SWITCHING_WINDOWS = true;
                should_swallow = true;
            }
            SwitchKeyAction::Cancel { id } if id == SWITCH_WINDOWS_HOTKEY_ID => {
                if let Some(message) = message_for_action(action) {
                    queued_messages.push(message);
                }
                IS_SWITCHING_WINDOWS = false;
                should_swallow = true;
            }
            _ => {}
        }
    }
    post_messages(WINDOW, &queued_messages);
    if should_swallow {
        return LRESULT(1);
    }
    CallNextHookEx(None, code, w_param, l_param)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_cancels_same_app_switching() {
        assert_eq!(
            switch_key_action(SWITCH_WINDOWS_HOTKEY_ID, 0x29, 0x01, false, true),
            Some(SwitchKeyAction::Cancel {
                id: SWITCH_WINDOWS_HOTKEY_ID
            })
        );
    }

    #[test]
    fn shift_reverses_same_app_switching() {
        assert_eq!(
            switch_key_action(SWITCH_WINDOWS_HOTKEY_ID, 0x29, 0x29, true, false),
            Some(SwitchKeyAction::Advance {
                id: SWITCH_WINDOWS_HOTKEY_ID,
                reverse: true
            })
        );
    }

    #[test]
    fn escape_is_ignored_before_switching_starts() {
        assert_eq!(
            switch_key_action(SWITCH_WINDOWS_HOTKEY_ID, 0x29, 0x01, false, false),
            None
        );
    }

    #[test]
    fn queued_messages_route_scalar_action_done_and_cancel_payloads() {
        assert_eq!(
            message_for_action(SwitchKeyAction::Advance {
                id: SWITCH_WINDOWS_HOTKEY_ID,
                reverse: true,
            }),
            Some(QueuedSwitchMessage {
                message: WM_USER_SWITCH_WINDOWS,
                wparam: 0,
                lparam: 1,
            })
        );
        assert_eq!(
            message_for_done(SWITCH_WINDOWS_HOTKEY_ID),
            Some(QueuedSwitchMessage {
                message: WM_USER_SWITCH_WINDOWS_DONE,
                wparam: 0,
                lparam: 0,
            })
        );
        assert_eq!(
            message_for_action(SwitchKeyAction::Cancel {
                id: SWITCH_APPS_HOTKEY_ID,
            }),
            Some(QueuedSwitchMessage {
                message: WM_USER_SWITCH_APPS_CANCEL,
                wparam: 0,
                lparam: 0,
            })
        );
    }

    #[test]
    fn dispatches_queued_messages_in_input_order() {
        let messages = [
            message_for_done(SWITCH_WINDOWS_HOTKEY_ID).unwrap(),
            message_for_action(SwitchKeyAction::Advance {
                id: SWITCH_APPS_HOTKEY_ID,
                reverse: false,
            })
            .unwrap(),
            message_for_action(SwitchKeyAction::Cancel {
                id: SWITCH_APPS_HOTKEY_ID,
            })
            .unwrap(),
        ];
        let mut received = Vec::new();

        dispatch_messages(&messages, |message| received.push(message));

        assert_eq!(received, messages);
    }
}
