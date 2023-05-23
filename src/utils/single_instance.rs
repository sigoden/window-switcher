use super::to_wstring;

use anyhow::{anyhow, Result};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, BOOL, ERROR_ALREADY_EXISTS, HANDLE};
use windows::Win32::System::Threading::{CreateMutexW, ReleaseMutex};

/// A struct representing one running instance.
pub struct SingleInstance {
    handle: Option<HANDLE>,
}

unsafe impl Send for SingleInstance {}
unsafe impl Sync for SingleInstance {}

impl SingleInstance {
    /// Returns a new SingleInstance object.
    pub fn create(name: &str) -> Result<Self> {
        let name = to_wstring(name);
        let handle = unsafe { CreateMutexW(None, BOOL(1), PCWSTR(name.as_ptr())) }
            .map_err(|err| anyhow!("Fail to setup single instance, {err}"))?;
        let handle =
            if windows::core::Error::from_win32().code() == ERROR_ALREADY_EXISTS.to_hresult() {
                None
            } else {
                Some(handle)
            };
        Ok(SingleInstance { handle })
    }

    /// Returns whether this instance is single.
    pub fn is_single(&self) -> bool {
        self.handle.is_some()
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            unsafe {
                ReleaseMutex(handle);
                CloseHandle(handle);
            }
        }
    }
}
