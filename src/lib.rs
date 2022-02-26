#[macro_use]
extern crate lazy_static;

mod startup;
mod trayicon;
mod utils;
mod windows;

pub use crate::trayicon::setup_trayicon;
pub use crate::utils::register_hotkey;
pub use crate::windows::switch_next_window;
