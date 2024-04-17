use anyhow::Result;
use windows::core::w;
use windows::core::PCWSTR;

use crate::utils::get_exe_path;
use crate::utils::RegKey;

const HKEY_RUN: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const HKEY_NAME: PCWSTR = w!("Window Switcher");

#[derive(Default)]
pub struct Startup {
    pub is_enable: bool,
}

impl Startup {
    pub fn init() -> Result<Self> {
        let enable = Self::detect()?;
        Ok(Self { is_enable: enable })
    }

    pub fn toggle(&mut self) -> Result<()> {
        let is_enable = self.is_enable;
        if is_enable {
            Self::disable()?;
            self.is_enable = false;
        } else {
            Self::enable()?;
            self.is_enable = true;
        }
        Ok(())
    }

    fn detect() -> Result<bool> {
        let key = win_run_key()?;
        let value = match key.get_value()? {
            Some(value) => value,
            None => return Ok(false),
        };
        let path = get_exe_path();
        Ok(value == path)
    }

    fn enable() -> Result<()> {
        let key = win_run_key()?;
        let path = get_exe_path();
        let path = unsafe { path.align_to::<u8>().1 };
        key.set_value(path)?;
        Ok(())
    }

    fn disable() -> Result<()> {
        let key = win_run_key()?;
        key.delete_value()?;
        Ok(())
    }
}

fn win_run_key() -> Result<RegKey> {
    RegKey::new_hkcu(HKEY_RUN, HKEY_NAME)
}
