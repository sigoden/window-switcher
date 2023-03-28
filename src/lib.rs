pub mod utils;
#[macro_use]
pub mod macros;
#[macro_use]
extern crate log;

mod app;
mod config;
mod foregound;
mod keyboard;
mod startup;
mod trayicon;

pub use crate::app::start;
pub use crate::config::Config;
