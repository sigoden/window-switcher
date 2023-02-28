#![windows_subsystem = "windows"]

use anyhow::{anyhow, Result};
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

use ini::Ini;
use window_switcher::{alert, start, utils::get_exe_folder, Config};

fn main() {
    if let Err(err) = run() {
        alert!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let config = load_config().unwrap_or_default();
    if let Some(log_file) = &config.log_file {
        let file = prepare_log_file(log_file).map_err(|err| {
            anyhow!(
                "Failed to prepare log file at {}, {err}",
                log_file.display()
            )
        })?;
        simple_logging::log_to(file, config.log_level);
    }
    start(&config)
}

fn load_config() -> Result<Config> {
    let folder = get_exe_folder()?;
    let ini_file = folder.join("window-switcher.ini");
    let conf =
        Ini::load_from_file(ini_file).map_err(|err| anyhow!("Faile to load ini file, {err}"))?;
    Config::load(&conf)
}

fn prepare_log_file(path: &Path) -> std::io::Result<File> {
    if path.exists() {
        OpenOptions::new().append(true).open(path)
    } else {
        File::create(path)
    }
}
