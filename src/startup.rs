use anyhow::{anyhow, bail, Result};
use windows::core::w;
use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegGetValueW, RegOpenKeyExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_ALL_ACCESS, REG_SZ, REG_VALUE_TYPE, RRF_RT_REG_SZ,
};

use crate::utils::get_exe_path;

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
        let key = get_key()?;
        let value = match get_value(&key.hkey)? {
            Some(value) => value,
            None => return Ok(false),
        };
        let path = get_exe_path();
        Ok(value == path)
    }

    fn enable() -> Result<()> {
        let key = get_key()?;
        let path = get_exe_path();
        let path_u8 = unsafe { path.align_to::<u8>().1 };
        unsafe { RegSetValueExW(key.hkey, HKEY_NAME, 0, REG_SZ, Some(path_u8)) }
            .map_err(|err| anyhow!("Fail to write reg value, {:?}", err))?;
        Ok(())
    }

    fn disable() -> Result<()> {
        let key = get_key()?;
        unsafe { RegDeleteValueW(key.hkey, HKEY_NAME) }
            .map_err(|err| anyhow!("Failed to delete reg value, {:?}", err))?;
        Ok(())
    }
}

struct WrapHKey {
    hkey: HKEY,
}

impl Drop for WrapHKey {
    fn drop(&mut self) {
        let _ = unsafe { RegCloseKey(self.hkey) };
    }
}

fn get_key() -> Result<WrapHKey> {
    let mut hkey = HKEY::default();
    unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            HKEY_RUN,
            0,
            KEY_ALL_ACCESS,
            &mut hkey as *mut _,
        )
    }
    .map_err(|err| anyhow!("Fail to open reg key, {:?}", err))?;
    Ok(WrapHKey { hkey })
}

fn get_value(hkey: &HKEY) -> Result<Option<Vec<u16>>> {
    let mut buffer = [0u16; 1024];
    let mut size: u32 = (1024 * std::mem::size_of_val(&buffer[0])) as u32;
    let mut kind: REG_VALUE_TYPE = Default::default();
    let ret = unsafe {
        RegGetValueW(
            *hkey,
            None,
            HKEY_NAME,
            RRF_RT_REG_SZ,
            Some(&mut kind),
            Some(buffer.as_mut_ptr() as *mut _),
            Some(&mut size),
        )
    };
    if let Err(err) = ret {
        if err.code().0 == ERROR_FILE_NOT_FOUND.0 as _ {
            return Ok(None);
        }
        bail!("Fail to get reg value, {:?}", err);
    }
    let len = (size as usize - 1) / 2;
    Ok(Some(buffer[..len].to_vec()))
}
