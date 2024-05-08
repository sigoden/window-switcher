use std::{collections::HashSet, fs, path::PathBuf, process::Command};

use anyhow::{anyhow, Result};
use ini::Ini;
use log::LevelFilter;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VIRTUAL_KEY, VK_LCONTROL, VK_LMENU, VK_LWIN, VK_RCONTROL, VK_RMENU, VK_RWIN,
};

use crate::utils::get_exe_folder;

pub const SWITCH_WINDOWS_HOTKEY_ID: u32 = 1;
pub const SWITCH_APPS_HOTKEY_ID: u32 = 2;

const DEFAULT_CONFIG: &str = include_str!("../window-switcher.ini");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub trayicon: bool,
    pub log_level: LevelFilter,
    pub log_file: Option<PathBuf>,
    pub switch_windows_hotkey: Hotkey,
    pub switch_windows_blacklist: HashSet<String>,
    pub switch_windows_ignore_minimal: bool,
    pub switch_apps_enable: bool,
    pub switch_apps_hotkey: Hotkey,
    pub switch_apps_ignore_minimal: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            trayicon: true,
            log_level: LevelFilter::Info,
            log_file: None,
            switch_windows_hotkey: Hotkey::create(
                SWITCH_WINDOWS_HOTKEY_ID,
                "switch windows",
                "alt + `",
            )
            .unwrap(),
            switch_windows_blacklist: Default::default(),
            switch_windows_ignore_minimal: false,
            switch_apps_enable: false,
            switch_apps_hotkey: Hotkey::create(SWITCH_APPS_HOTKEY_ID, "switch apps", "alt + tab")
                .unwrap(),
            switch_apps_ignore_minimal: false,
        }
    }
}

impl Config {
    pub fn load(ini_conf: &Ini) -> Result<Self> {
        let mut conf = Config::default();
        if let Some(section) = ini_conf.section(None::<String>) {
            if let Some(v) = section.get("trayicon").and_then(Config::to_bool) {
                conf.trayicon = v;
            }
        }

        if let Some(section) = ini_conf.section(Some("log")) {
            if let Some(level) = section.get("level").and_then(|v| v.parse().ok()) {
                conf.log_level = level;
            }
            if let Some(path) = section.get("path") {
                if !path.trim().is_empty() {
                    let mut path = PathBuf::from(path);
                    if !path.is_absolute() {
                        let parent = get_exe_folder()?;
                        path = parent.join(path);
                    }
                    conf.log_file = Some(path);
                }
            }
        }

        if let Some(section) = ini_conf.section(Some("switch-windows")) {
            if let Some(v) = section.get("hotkey") {
                if !v.trim().is_empty() {
                    conf.switch_windows_hotkey =
                        Hotkey::create(SWITCH_WINDOWS_HOTKEY_ID, "switch windows", v)?;
                }
            }

            if let Some(v) = section
                .get("blacklist")
                .map(|v| v.split(',').map(|v| v.trim().to_string()).collect())
            {
                conf.switch_windows_blacklist = v;
            }
            if let Some(v) = section.get("ignore_minimal").and_then(Config::to_bool) {
                conf.switch_windows_ignore_minimal = v;
            }
        }
        if let Some(section) = ini_conf.section(Some("switch-apps")) {
            if let Some(v) = section.get("enable").and_then(Config::to_bool) {
                conf.switch_apps_enable = v;
            }
            if let Some(v) = section.get("hotkey") {
                if !v.trim().is_empty() {
                    conf.switch_apps_hotkey =
                        Hotkey::create(SWITCH_APPS_HOTKEY_ID, "switch apps", v)?;
                }
            }
            if let Some(v) = section.get("ignore_minimal").and_then(Config::to_bool) {
                conf.switch_apps_ignore_minimal = v;
            }
        }
        Ok(conf)
    }

    pub fn to_hotkeys(&self) -> Vec<&Hotkey> {
        let mut hotkeys = vec![&self.switch_windows_hotkey];
        if self.switch_apps_enable {
            hotkeys.push(&self.switch_apps_hotkey);
        }
        hotkeys
    }

    pub fn to_bool(v: &str) -> Option<bool> {
        match v {
            "yes" | "true" | "on" | "1" => Some(true),
            "no" | "false" | "off" | "0" => Some(false),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hotkey {
    pub id: u32,
    pub name: String,
    pub modifier: [VIRTUAL_KEY; 2],
    pub code: u16,
}

impl Hotkey {
    pub fn create(id: u32, name: &str, value: &str) -> Result<Self> {
        let (modifier, code) =
            Self::parse(value).ok_or_else(|| anyhow!("Invalid {name} hotkey"))?;
        Ok(Self {
            id,
            name: name.to_string(),
            modifier,
            code,
        })
    }

    pub fn get_modifier(&self) -> u16 {
        self.modifier[0].0
    }

    pub fn parse(value: &str) -> Option<([VIRTUAL_KEY; 2], u16)> {
        let value = value.to_ascii_lowercase().replace(' ', "");
        let keys: Vec<&str> = value.split('+').collect();
        if keys.len() != 2 {
            return None;
        }
        let modifier = match keys[0] {
            "win" => [VK_LWIN, VK_RWIN],
            "alt" => [VK_LMENU, VK_RMENU],
            "ctrl" => [VK_LCONTROL, VK_RCONTROL],
            _ => {
                return None;
            }
        };
        let code = match keys[1] {
            "backspace" => 0x08,
            "tab" => 0x09,
            "clear" => 0x0c,
            "enter" => 0x0d,
            "pause" => 0x13,
            "capslock" => 0x14,
            "escape" => 0x1b,
            "space" => 0x20,
            "pageup" => 0x21,
            "pagedown" => 0x22,
            "end" => 0x23,
            "home" => 0x24,
            "left" => 0x25,
            "up" => 0x26,
            "right" => 0x27,
            "down" => 0x28,
            "select" => 0x29,
            "print" => 0x2a,
            "printscreen" => 0x2c,
            "insert" => 0x2d,
            "delete" => 0x2e,

            "0" => 0x30,
            "1" => 0x31,
            "2" => 0x32,
            "3" => 0x33,
            "4" => 0x34,
            "5" => 0x35,
            "6" => 0x36,
            "7" => 0x37,
            "8" => 0x38,
            "9" => 0x39,
            "a" => 0x41,
            "b" => 0x42,
            "c" => 0x43,
            "d" => 0x44,
            "e" => 0x45,
            "f" => 0x46,
            "g" => 0x47,
            "h" => 0x48,
            "i" => 0x49,
            "j" => 0x4a,
            "k" => 0x4b,
            "l" => 0x4c,
            "m" => 0x4d,
            "n" => 0x4e,
            "o" => 0x4f,
            "p" => 0x50,
            "q" => 0x51,
            "r" => 0x52,
            "s" => 0x53,
            "t" => 0x54,
            "u" => 0x55,
            "v" => 0x56,
            "w" => 0x57,
            "x" => 0x58,
            "y" => 0x59,
            "z" => 0x5a,

            "f1" => 0x70,
            "f2" => 0x71,
            "f3" => 0x72,
            "f4" => 0x73,
            "f5" => 0x74,
            "f6" => 0x75,
            "f7" => 0x76,
            "f8" => 0x77,
            "f9" => 0x78,
            "f10" => 0x79,
            "f11" => 0x7a,
            "f12" => 0x7b,
            "numlock" => 0x90,
            "scrolllock" => 0x91,

            ":" | ";" | "vk_oem_1" => 0xba,
            "+" | "=" | "vk_oem_plus" => 0xbb,
            "<" | "," | "vk_oem_comma" => 0xbc,
            "-" | "_" | "vk_oem_minus" => 0xbd,
            ">" | "." | "vk_oem_period" => 0xbe,
            "?" | "/" | "vk_oem_2" => 0xbf,
            "~" | "`" | "vk_oem_3" => 0xc0,
            "{" | "[" | "vk_oem_4" => 0xdb,
            "|" | "\\" | "vk_oem_5" => 0xdc,
            "}" | "]" | "vk_oem_6" => 0xdd,
            "\"" | "'" | "vk_oem_7" => 0xde,
            "§" | "!" | "vk_oem_8" => 0xdf,
            _ => return None,
        };
        Some((modifier, code))
    }
}

pub fn load_config() -> Result<Config> {
    let filepath = get_config_path()?;
    let conf = Ini::load_from_file(&filepath)
        .map_err(|err| anyhow!("Failed to load config file '{}', {err}", filepath.display()))?;
    Config::load(&conf)
}

pub(crate) fn edit_config_file() -> Result<bool> {
    let filepath = get_config_path()?;
    debug!("open config file '{}'", filepath.display());
    if !filepath.exists() {
        fs::write(&filepath, DEFAULT_CONFIG).map_err(|err| {
            anyhow!(
                "Failed to write config file '{}', {err}",
                filepath.display()
            )
        })?;
    }
    let exit = Command::new("notepad.exe")
        .arg(&filepath)
        .spawn()
        .map_err(|err| anyhow!("Failed to open config file '{}', {err}", filepath.display()))?
        .wait()
        .map_err(|err| {
            anyhow!(
                "Failed to close config file '{}', {err}",
                filepath.display()
            )
        })?;

    Ok(exit.success())
}

fn get_config_path() -> Result<PathBuf> {
    let folder = get_exe_folder()?;
    let config_path = folder.join("window-switcher.ini");
    Ok(config_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotkey() {
        assert_eq!(Hotkey::parse("alt + `"), Some(([VK_LMENU, VK_RMENU], 0xc0)));
        assert_eq!(
            Hotkey::parse("alt + tab"),
            Some(([VK_LMENU, VK_RMENU], 0x09))
        );
    }
}
