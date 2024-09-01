use super::HandleWrapper;

use anyhow::{anyhow, Result};
use windows::Win32::{
    Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

pub fn is_running_as_admin() -> Result<bool> {
    is_running_as_admin_impl()
        .map_err(|err| anyhow!("Failed to verify if the program is running as admin, {err}"))
}

fn is_running_as_admin_impl() -> Result<bool> {
    let is_elevated = unsafe {
        let mut token_handle = HandleWrapper::default();
        let mut elevation = TOKEN_ELEVATION::default();
        let mut returned_length = 0;
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_QUERY,
            token_handle.get_handle_mut(),
        )?;

        GetTokenInformation(
            token_handle.get_handle(),
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned_length,
        )?;

        elevation.TokenIsElevated != 0
    };
    Ok(is_elevated)
}
