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
    let sec = conf.section(None::<String>)?;
    let default_config = Config::default();
    let trayicon = sec
        .get("trayicon")
        .and_then(Config::to_bool)
        .unwrap_or(default_config.trayicon);
    let hotkey = sec
        .get("hotkey")
        .and_then(Config::to_hotkey)
        .unwrap_or(default_config.hotkey);
    Some(Config { trayicon, hotkey })
}
