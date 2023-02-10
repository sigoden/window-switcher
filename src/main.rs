#![windows_subsystem = "windows"]

use ini::Ini;
use windows_switcher::{start_app, Config};

fn main() {
    let config = load_config().unwrap_or_default();
    start_app(&config);
}

fn load_config() -> Option<Config> {
    let mut ini_file = std::env::current_exe().ok()?;
    ini_file.pop();
    ini_file.push("windows-switcher.ini");
    let conf = Ini::load_from_file(ini_file).ok()?;
    Some(Config::load(&conf))
}
