use crate::utils::get_window_exe;
use anyhow::{bail, Result};
use once_cell::sync::OnceCell;
use std::collections::HashSet;
use windows::Win32::{
    Foundation::HWND,
    UI::{
        Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK},
        WindowsAndMessaging::{
            EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
        },
    },
};

pub static mut IS_FOREGROUND_IN_BLACKLIST: bool = false;

static BLACKLIST: OnceCell<HashSet<String>> = OnceCell::new();

#[derive(Debug)]
pub struct ForegroundWatcher {
    hook: HWINEVENTHOOK,
}

impl ForegroundWatcher {
    pub fn init(blacklist: &HashSet<String>) -> Result<Self> {
        if blacklist.is_empty() {
            return Ok(Self {
                hook: HWINEVENTHOOK::default(),
            });
        }

        let _ = BLACKLIST.set(blacklist.clone());

        let hook = unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(win_event_proc),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            )
        };
        if hook.is_invalid() {
            bail!("Failed to watch foreground");
        }
        Ok(Self { hook })
    }
}

impl Drop for ForegroundWatcher {
    fn drop(&mut self) {
        debug!("foreground watcher destoryed");
        if !self.hook.is_invalid() {
            unsafe { UnhookWinEvent(self.hook) };
        }
    }
}

unsafe extern "system" fn win_event_proc(
    _h_win_event_hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _dw_event_thread: u32,
    _dwms_event_time: u32,
) {
    let exe = get_window_exe(hwnd);
    if exe.is_empty() {
        return;
    }
    IS_FOREGROUND_IN_BLACKLIST = BLACKLIST.get().unwrap().contains(&exe);
    debug!("foreground {exe} {IS_FOREGROUND_IN_BLACKLIST}");
}
