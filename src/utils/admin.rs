use super::HandleWrapper;

use anyhow::{anyhow, Result};
use windows::Win32::{
    Foundation::HANDLE,
    Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
    System::Threading::{
        GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    },
};

pub fn is_running_as_admin() -> Result<bool> {
    let process = unsafe { GetCurrentProcess() };
    is_elevated(process)
        .map_err(|err| anyhow!("Failed to verify if the program is running as admin, {err}"))
}

pub fn is_process_elevated(pid: u32) -> Option<bool> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    is_elevated(handle).ok()
}

pub fn is_elevated(handle: HANDLE) -> Result<bool> {
    let is_elevated = unsafe {
        let mut token_handle = HandleWrapper::default();
        let mut elevation = TOKEN_ELEVATION::default();
        let mut returned_length = 0;
        OpenProcessToken(handle, TOKEN_QUERY, token_handle.get_handle_mut())?;

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
