#![windows_subsystem = "windows"]

use ini::Ini;
use windows_switcher::{start_app, Config, HotKeyConfig};

fn main() {
    let config = load_config().unwrap_or_default();
    start_app(&config);
}

fn load_config() -> Option<Config> {
    let mut ini_file = std::env::current_exe().ok()?;
    ini_file.pop();
    ini_file.push("windows-switcher.ini");
    let conf = Ini::load_from_file(ini_file).ok()?;
    let section = conf.section(None::<String>)?;
    let default_config = Config::default();
    let trayicon = section
        .get("trayicon")
        .and_then(Config::to_bool)
        .unwrap_or(default_config.trayicon);
    let hotkey = section
        .get("hotkey")
        .and_then(HotKeyConfig::parse)
        .unwrap_or(default_config.hotkey);
    let blacklist = section
        .get("blacklist")
        .map(|v| {
            let v = v.trim();
            if v.is_empty() {
                String::new()
            } else {
                format!(",{}", v.trim().to_lowercase())
            }
        })
        .unwrap_or_default();
    Some(Config {
        trayicon,
        hotkey,
        blacklist,
    })
}
