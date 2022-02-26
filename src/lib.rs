#[macro_use]
extern crate lazy_static;

mod trayicon;
mod utils;
mod windows;

const TRAYICON_TOOLTIP: &str = "Windows Swither On";
const TRAYICON_ICON_BUFFER: &[u8] = include_bytes!("../assets/icon.ico");

pub use crate::trayicon::setup_trayicon;
pub use crate::utils::register_hotkey;
pub use crate::windows::switch_next_window;
