pub mod utils;
#[macro_use]
pub mod macros;
#[macro_use]
extern crate log;

mod app;
mod config;
mod foreground;
mod keyboard;
mod painter;
mod startup;
mod thumbnail;
mod trayicon;
pub mod window_grid;

pub use crate::app::start;
pub use crate::config::{load_config, Config};
