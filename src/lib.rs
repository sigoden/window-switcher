mod app;
mod startup;
mod switcher;
mod trayicon;
mod utils;

#[macro_use]
mod macros;
mod config;

use windows::core::Error as Win32Error;
use windows::core::Result as Win32Result;

pub use crate::app::start_app;
pub use crate::config::{Config, HotKeyConfig};
