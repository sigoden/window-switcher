use anyhow::{anyhow, bail, Result};
use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegGetValueW, RegOpenKeyExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_ALL_ACCESS, REG_DWORD_BIG_ENDIAN, REG_SZ, REG_VALUE_TYPE,
    RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
};

#[derive(Debug)]
pub struct RegKey {
    hkey: HKEY,
    name: PCWSTR,
}

impl RegKey {
    pub fn new_hkcu(subkey: PCWSTR, name: PCWSTR) -> Result<RegKey> {
        let mut hkey = HKEY::default();
        unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                subkey,
                0,
                KEY_ALL_ACCESS,
                &mut hkey as *mut _,
            )
        }
        .ok()
        .map_err(|err| anyhow!("Fail to open reg key, {:?}", err))?;
        Ok(RegKey { hkey, name })
    }

    pub fn get_value(&self) -> Result<Option<Vec<u16>>> {
        let mut buffer = [0u16; 1024];
        let mut size: u32 = (1024 * std::mem::size_of_val(&buffer[0])) as u32;
        let mut kind: REG_VALUE_TYPE = Default::default();
        let ret = unsafe {
            RegGetValueW(
                self.hkey,
                None,
                self.name,
                RRF_RT_REG_SZ,
                Some(&mut kind),
                Some(buffer.as_mut_ptr() as *mut _),
                Some(&mut size),
            )
        };
        if ret.is_err() {
            if ret == ERROR_FILE_NOT_FOUND {
                return Ok(None);
            }
            bail!(
                "Fail to get reg value, {:?}",
                windows::core::Error::from(ret)
            );
        }
        let len = (size as usize - 1) / 2;
        Ok(Some(buffer[..len].to_vec()))
    }

    pub fn get_int(&self) -> Result<u32> {
        let mut value: [u8; 4] = Default::default();
        let mut size: u32 = std::mem::size_of_val(&value) as u32;
        let mut kind: REG_VALUE_TYPE = Default::default();
        let ret = unsafe {
            RegGetValueW(
                self.hkey,
                None,
                self.name,
                RRF_RT_REG_DWORD,
                Some(&mut kind),
                Some(value.as_mut_ptr() as *mut _),
                Some(&mut size),
            )
        };
        if ret.is_err() {
            bail!(
                "Fail to get reg value, {:?}",
                windows::core::Error::from(ret)
            );
        }
        let value = if kind == REG_DWORD_BIG_ENDIAN {
            u32::from_be_bytes(value)
        } else {
            u32::from_le_bytes(value)
        };
        Ok(value)
    }

    pub fn set_value(&self, value: &[u8]) -> Result<()> {
        unsafe { RegSetValueExW(self.hkey, self.name, 0, REG_SZ, Some(value)) }
            .ok()
            .map_err(|err| anyhow!("Fail to write reg value, {:?}", err))?;
        Ok(())
    }

    pub fn delete_value(&self) -> Result<()> {
        unsafe { RegDeleteValueW(self.hkey, self.name) }
            .ok()
            .map_err(|err| anyhow!("Failed to delete reg value, {:?}", err))?;
        Ok(())
    }
}

impl Drop for RegKey {
    fn drop(&mut self) {
        let _ = unsafe { RegCloseKey(self.hkey) };
    }
}
