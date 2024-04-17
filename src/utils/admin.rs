use anyhow::{anyhow, Result};
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

pub fn is_running_as_admin() -> Result<bool> {
    is_running_as_admin_impl()
        .map_err(|err| anyhow!("Failed to verify if the program is running as admin, {err}"))
}

fn is_running_as_admin_impl() -> Result<bool> {
    let is_elevated = unsafe {
        let mut token_handle: HANDLE = HANDLE(0);
        let mut elevation = TOKEN_ELEVATION::default();
        let mut returned_length = 0;
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle)?;

        let token_information = GetTokenInformation(
            token_handle,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned_length,
        );

        CloseHandle(token_handle)?;

        token_information?;
        elevation.TokenIsElevated != 0
    };
    Ok(is_elevated)
}
