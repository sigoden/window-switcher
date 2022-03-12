mod app;
mod startup;
mod switcher;
mod trayicon;
#[macro_use]
mod macros;

use windows::core::Error as Win32Error;
use windows::core::Result as Win32Result;

pub use crate::app::start_app;
