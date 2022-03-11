use anyhow::{anyhow, Result};
use std::ops::Deref;
use std::ptr::null_mut;
use windows::core::GUID;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CoCreateInstance, CoInitialize, CoUninitialize, CLSCTX_ALL};
use windows::Win32::UI::Shell::IVirtualDesktopManager;

#[allow(non_upper_case_globals)]
const CLSID_VirtualDesktopManager: GUID = GUID::from_u128(0xaa509086_5ca9_4c25_8f95_589d3c07b48a);

pub struct Com();

impl Com {
    pub fn create() -> Result<Self> {
        unsafe {
            CoInitialize(null_mut()).map_err(|e| anyhow!("Fail to init com, {}", e))?;
        }
        Ok(Self())
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        unsafe { CoUninitialize() }
    }
}

#[derive(Clone)]
pub struct VirtualDesktop {
    inner: IVirtualDesktopManager,
}

impl VirtualDesktop {
    pub fn create() -> Result<Self> {
        let inner = unsafe {
            CoCreateInstance(&CLSID_VirtualDesktopManager, None, CLSCTX_ALL)
                .map_err(|e| anyhow!("Fail to access virtual desktop com, {}", e))?
        };
        Ok(Self { inner })
    }
    pub fn is_window_on_current_virtual_desktop(&self, window: HWND) -> Result<bool> {
        let ret = unsafe { self.inner.IsWindowOnCurrentVirtualDesktop(window) }
            .map_err(|e| anyhow!("Fail to check current desktop, {}", e))?;
        Ok(ret.as_bool())
    }
}

impl Deref for VirtualDesktop {
    type Target = IVirtualDesktopManager;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
