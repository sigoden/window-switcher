mod app;
mod startup;
mod switcher;
mod trayicon;
#[macro_use]
mod macros;
mod config;
mod utils;

pub use crate::app::start;
pub use crate::config::{Config, HotKeyConfig};
