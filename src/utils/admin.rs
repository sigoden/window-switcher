use super::HandleWrapper;

use anyhow::{anyhow, Result};
use windows::Win32::{
    Foundation::HANDLE,
    Security::{
        GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TokenElevation,
        TokenElevationType, TokenElevationTypeFull, TokenIntegrityLevel, TOKEN_ELEVATION,
        TOKEN_ELEVATION_TYPE, TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
    },
    System::Threading::{
        GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    },
};

const SECURITY_MANDATORY_HIGH_RID: u32 = 0x00003000;
const SECURITY_MANDATORY_SYSTEM_RID: u32 = 0x00004000;

pub fn is_running_as_admin() -> Result<bool> {
    let process = unsafe { GetCurrentProcess() };
    is_elevated(process)
        .map_err(|err| anyhow!("Failed to verify if the program is running as admin, {err}"))
}

pub fn is_process_elevated(pid: u32) -> Option<bool> {
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    get_process_elevation_info(process).ok()
}

fn get_process_elevation_info(process: HANDLE) -> Result<bool> {
    unsafe {
        let mut token = HandleWrapper::default();
        OpenProcessToken(process, TOKEN_QUERY, token.get_handle_mut())?;

        query_token_elevated(token.get_handle())
    }
}

unsafe fn query_token_elevated(token: HANDLE) -> Result<bool> {
    let mut ret_len = 0u32;

    let mut elevation = TOKEN_ELEVATION::default();
    GetTokenInformation(
        token,
        TokenElevation,
        Some(&mut elevation as *mut _ as *mut _),
        std::mem::size_of::<TOKEN_ELEVATION>() as u32,
        &mut ret_len,
    )?;

    let mut elevation_type = TOKEN_ELEVATION_TYPE(0);
    GetTokenInformation(
        token,
        TokenElevationType,
        Some(&mut elevation_type as *mut _ as *mut _),
        std::mem::size_of::<TOKEN_ELEVATION_TYPE>() as u32,
        &mut ret_len,
    )?;

    let mut buf = [0u8; 512];
    GetTokenInformation(
        token,
        TokenIntegrityLevel,
        Some(buf.as_mut_ptr() as *mut _),
        buf.len() as u32,
        &mut ret_len,
    )?;

    let label = &*(buf.as_ptr() as *const TOKEN_MANDATORY_LABEL);
    let sid = label.Label.Sid;
    if sid.0.is_null() {
        return Err(anyhow!("SID is null"));
    }
    let sub_auth_count = *GetSidSubAuthorityCount(sid);
    let rid = *GetSidSubAuthority(sid, (sub_auth_count - 1).into());

    Ok(matches!(
        rid,
        SECURITY_MANDATORY_HIGH_RID | SECURITY_MANDATORY_SYSTEM_RID
    ) && elevation.TokenIsElevated != 0
        && elevation_type == TokenElevationTypeFull)
}

pub fn is_elevated(handle: HANDLE) -> Result<bool> {
    get_process_elevation_info(handle)
}
