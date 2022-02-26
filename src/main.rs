#![windows_subsystem = "windows"]

use windows_switcher::{register_hotkey, setup_trayicon, switch_next_window};

use windows::{
    Win32::UI::WindowsAndMessaging::DispatchMessageW,
    Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY},
};

fn main() {
    setup_trayicon();
    register_hotkey();
    eventloop();
}

fn eventloop() {
    unsafe {
        let mut msg = MSG::default();
        loop {
            let res = GetMessageW(&mut msg, None, 0, 0);
            if res.as_bool() {
                if msg.message == WM_HOTKEY {
                    switch_next_window();
                }
                DispatchMessageW(&msg);
            } else {
                break;
            }
        }
    }
}
