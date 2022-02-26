use anyhow::{bail, Result};
use windows::Win32::{
    Foundation::{ERROR_SUCCESS, MAX_PATH, PWSTR},
    System::{
        LibraryLoader::GetModuleFileNameW,
        Registry::{
            RegCloseKey, RegDeleteValueW, RegGetValueW, RegOpenKeyExW, RegSetValueExW, HKEY,
            HKEY_CURRENT_USER, KEY_ALL_ACCESS, REG_SZ, RRF_RT_REG_SZ,
        },
    },
};

use crate::utils::{output_debug, wchar};

const HKEY_RUN: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const HKEY_NAME: &str = "WindowsSwitcher";

#[derive(Default)]
pub struct Startup {
    enable: Option<bool>,
}

impl Startup {
    pub fn check(&mut self) -> bool {
        match self.enable {
            Some(enable) => enable,
            None => match check_startup() {
                Ok(enable) => {
                    self.enable = Some(enable);
                    enable
                }
                Err(err) => {
                    output_debug(&format!("Startup: {}", err));
                    false
                }
            },
        }
    }
    pub fn toggle(&mut self) {
        let is_enable = self.check();
        let ret = {
            if is_enable {
                disable_startup()
            } else {
                enable_startup()
            }
        };
        match ret {
            Ok(_) => {
                self.enable = Some(!is_enable);
            }
            Err(err) => {
                output_debug(&format!("Startup: {}", err));
            }
        }
    }
}

fn check_startup() -> Result<bool> {
    let key = get_key()?;
    let value = get_value(&key.hkey)?;
    let path = get_exe_path();
    Ok(value == path)
}

fn enable_startup() -> Result<()> {
    let key = get_key()?;
    let name = wchar(HKEY_NAME);
    let path = get_exe_path();
    let ret = unsafe {
        RegSetValueExW(
            &key.hkey,
            PWSTR(name.as_ptr()),
            0,
            REG_SZ,
            path.as_ptr() as *const _,
            path.len() as u32,
        )
    };
    if ret != ERROR_SUCCESS {
        bail!("Fail to write reg value, {:?}", ret);
    }
    Ok(())
}

fn disable_startup() -> Result<()> {
    let key = get_key()?;
    let name = wchar(HKEY_NAME);
    unsafe { RegDeleteValueW(&key.hkey, PWSTR(name.as_ptr())) };
    Ok(())
}

fn get_value(hkey: &HKEY) -> Result<Vec<u16>> {
    let name = wchar(HKEY_NAME);
    let mut len: u32 = MAX_PATH;
    let mut value = vec![0u16; len as usize];
    let mut value_type: u32 = 0;
    let ret = unsafe {
        RegGetValueW(
            hkey,
            None,
            PWSTR(name.as_ptr()),
            RRF_RT_REG_SZ,
            &mut value_type,
            value.as_mut_ptr() as *mut _,
            &mut len as *mut _,
        )
    };
    if ret != ERROR_SUCCESS {
        bail!("Fail to get reg value, {:?}", ret);
    }
    Ok(value)
}

struct WrapHKey {
    hkey: HKEY,
}

impl Drop for WrapHKey {
    fn drop(&mut self) {
        unsafe { RegCloseKey(self.hkey) };
    }
}

fn get_key() -> Result<WrapHKey> {
    let mut hkey = HKEY::default();
    let subkey = wchar(HKEY_RUN);
    let ret = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PWSTR(subkey.as_ptr()),
            0,
            KEY_ALL_ACCESS,
            &mut hkey as *mut _,
        )
    };
    if ret != ERROR_SUCCESS {
        bail!("Fail to open reg key, {:?}", ret);
    }
    Ok(WrapHKey { hkey })
}

fn get_exe_path() -> Vec<u16> {
    let path = vec![0u16; MAX_PATH as usize];
    unsafe { GetModuleFileNameW(None, PWSTR(path.as_ptr()), path.len() as u32) };
    path
}
