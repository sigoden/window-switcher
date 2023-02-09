use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, VIRTUAL_KEY, VK_CONTROL, VK_LWIN,
    VK_MENU, VK_SHIFT,
};

#[derive(Debug, Clone)]
pub struct Config {
    pub trayicon: bool,
    pub hotkey: HotKeyConfig,
    pub blacklist: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            trayicon: true,
            hotkey: Default::default(),
            blacklist: Default::default(),
        }
    }
}

impl Config {
    pub fn to_bool(v: &str) -> Option<bool> {
        match v {
            "yes" | "true" | "on" | "1" => Some(true),
            "no" | "false" | "off" | "0" => Some(false),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotKeyConfig {
    pub modifier: HOT_KEY_MODIFIERS,
    pub code: u32,
    pub vk: VIRTUAL_KEY,
}

impl Default for HotKeyConfig {
    fn default() -> Self {
        Self::new(MOD_ALT, 0xC0, VK_MENU)
    }
}

impl HotKeyConfig {
    pub fn new(modifier: HOT_KEY_MODIFIERS, code: u32, vk: VIRTUAL_KEY) -> Self {
        Self { modifier, code, vk }
    }

    pub fn parse(value: &str) -> Option<Self> {
        let value = value.to_ascii_lowercase().replace(' ', "");
        let mut modifier: HOT_KEY_MODIFIERS = Default::default();
        let mut code = 0;
        let mut vk: VIRTUAL_KEY = Default::default();
        for (i, v) in value.split('+').enumerate() {
            match v {
                "win" => {
                    modifier |= MOD_WIN;
                    if i == 0 {
                        vk = VK_LWIN
                    }
                }
                "alt" => {
                    modifier |= MOD_ALT;
                    if i == 0 {
                        vk = VK_MENU
                    }
                }
                "shift" => {
                    modifier |= MOD_SHIFT;
                    if i == 0 {
                        vk = VK_SHIFT
                    }
                }
                "ctrl" => {
                    modifier |= MOD_CONTROL;
                    if i == 0 {
                        vk = VK_CONTROL
                    }
                }
                _ => {
                    if code != 0 {
                        return None;
                    }
                    match v {
                        "backspace" => code = 0x08,
                        "tab" => code = 0x09,
                        "clear" => code = 0x0c,
                        "enter" => code = 0x0d,
                        "pause" => code = 0x13,
                        "capslock" => code = 0x14,
                        "escape" => code = 0x1b,
                        "space" => code = 0x20,
                        "pageup" => code = 0x21,
                        "pagedown" => code = 0x22,
                        "end" => code = 0x23,
                        "home" => code = 0x24,
                        "left" => code = 0x25,
                        "up" => code = 0x26,
                        "right" => code = 0x27,
                        "down" => code = 0x28,
                        "select" => code = 0x29,
                        "print" => code = 0x2a,
                        "printscreen" => code = 0x2c,
                        "insert" => code = 0x2d,
                        "delete" => code = 0x2e,

                        "0" => code = 0x30,
                        "1" => code = 0x31,
                        "2" => code = 0x32,
                        "3" => code = 0x33,
                        "4" => code = 0x34,
                        "5" => code = 0x35,
                        "6" => code = 0x36,
                        "7" => code = 0x37,
                        "8" => code = 0x38,
                        "9" => code = 0x39,
                        "a" => code = 0x41,
                        "b" => code = 0x42,
                        "c" => code = 0x43,
                        "d" => code = 0x44,
                        "e" => code = 0x45,
                        "f" => code = 0x46,
                        "g" => code = 0x47,
                        "h" => code = 0x48,
                        "i" => code = 0x49,
                        "j" => code = 0x4a,
                        "k" => code = 0x4b,
                        "l" => code = 0x4c,
                        "m" => code = 0x4d,
                        "n" => code = 0x4e,
                        "o" => code = 0x4f,
                        "p" => code = 0x50,
                        "q" => code = 0x51,
                        "r" => code = 0x52,
                        "s" => code = 0x53,
                        "t" => code = 0x54,
                        "u" => code = 0x55,
                        "v" => code = 0x56,
                        "w" => code = 0x57,
                        "x" => code = 0x58,
                        "y" => code = 0x59,
                        "z" => code = 0x5a,

                        "f1" => code = 0x70,
                        "f2" => code = 0x71,
                        "f3" => code = 0x72,
                        "f4" => code = 0x73,
                        "f5" => code = 0x74,
                        "f6" => code = 0x75,
                        "f7" => code = 0x76,
                        "f8" => code = 0x77,
                        "f9" => code = 0x78,
                        "f10" => code = 0x79,
                        "f11" => code = 0x7a,
                        "f12" => code = 0x7b,
                        "numlock" => code = 0x90,
                        "scrolllock" => code = 0x91,

                        ":" | ";" => code = 0xba,
                        "+" | "=" => code = 0xbb,
                        "<" | "," => code = 0xbc,
                        "-" | "_" => code = 0xbd,
                        ">" | "." => code = 0xbe,
                        "?" | "/" => code = 0xbf,
                        "{" | "[" => code = 0xdb,
                        "|" | "\\" => code = 0xdc,
                        "}" | "]" => code = 0xdd,
                        "\"" | "'" => code = 0xde,
                        "§" | "!" => code = 0xdf,
                        "~" | "`" => code = 0xc0,

                        _ => return None,
                    }
                }
            }
        }
        Some(Self::new(modifier, code, vk))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotkey() {
        assert_eq!(
            HotKeyConfig::parse("alt + `"),
            Some(HotKeyConfig::new(MOD_ALT, 0xc0, VK_MENU))
        );
        assert_eq!(
            HotKeyConfig::parse("ctrl + shift + `"),
            Some(HotKeyConfig::new(MOD_CONTROL | MOD_SHIFT, 0xc0, VK_CONTROL))
        );
    }
}
