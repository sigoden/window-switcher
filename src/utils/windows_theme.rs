use windows::core::w;

use super::RegKey;

pub fn is_light_theme() -> bool {
    let Ok(reg_key) = RegKey::new_hkcu(
        w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
        w!("SystemUsesLightTheme"),
    ) else {
        return false;
    };
    reg_key.get_int().map(|v| v == 1).unwrap_or(false)
}
