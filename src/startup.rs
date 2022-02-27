use anyhow::{bail, Result};
use wchar::{wchar_t, wchz};
use windows::Win32::{
    Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, MAX_PATH, PWSTR},
    System::{
        LibraryLoader::GetModuleFileNameW,
        Registry::{
            RegCloseKey, RegDeleteValueW, RegGetValueW, RegOpenKeyExW, RegSetValueExW, HKEY,
            HKEY_CURRENT_USER, KEY_ALL_ACCESS, REG_SZ, RRF_RT_REG_SZ,
        },
    },
};

const HKEY_RUN: &[wchar_t] = wchz!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const HKEY_NAME: &[wchar_t] = wchz!("Windows Switcher");

#[derive(Default)]
pub struct Startup {
    pub is_enable: bool,
}

impl Startup {
    pub fn create() -> Result<Self> {
        let enable = check()?;
        Ok(Self { is_enable: enable })
    }
    pub fn toggle(&mut self) -> Result<()> {
        let is_enable = self.is_enable;
        if is_enable {
            disable()?;
            self.is_enable = false;
        } else {
            enable()?;
            self.is_enable = true;
        }
        Ok(())
    }
}

fn check() -> Result<bool> {
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
    let ret = unsafe {
        RegSetValueExW(
            &key.hkey,
            PWSTR(HKEY_NAME.as_ptr()),
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

fn disable() -> Result<()> {
    let key = get_key()?;
    let ret = unsafe { RegDeleteValueW(&key.hkey, PWSTR(HKEY_NAME.as_ptr())) };
    if ret != ERROR_SUCCESS {
        bail!("Fail to delele reg value, {:?}", ret);
    }
    Ok(())
}

fn get_value(hkey: &HKEY) -> Result<Option<Vec<u16>>> {
    let mut len: u32 = MAX_PATH;
    let mut value = vec![0u16; len as usize];
    let mut value_type: u32 = 0;
    let ret = unsafe {
        RegGetValueW(
            hkey,
            None,
            PWSTR(HKEY_NAME.as_ptr()),
            RRF_RT_REG_SZ,
            &mut value_type,
            value.as_mut_ptr() as *mut _,
            &mut len as *mut _,
        )
    };
    if ret != ERROR_SUCCESS {
        if ret == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        bail!("Fail to get reg value, {:?}", ret);
    }
    Ok(Some(value))
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
    let ret = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PWSTR(HKEY_RUN.as_ptr()),
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
