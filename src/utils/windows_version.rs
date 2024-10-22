use windows::{
    Wdk::System::SystemServices::RtlGetVersion,
    Win32::System::SystemInformation::{OSVERSIONINFOEXW, OSVERSIONINFOW},
};

pub fn os_version_info() -> Option<OSVERSIONINFOW> {
    let mut info = OSVERSIONINFOW {
        dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOEXW>() as _,
        ..Default::default()
    };

    let status = unsafe { RtlGetVersion(&mut info) };
    if status.is_ok() {
        Some(info)
    } else {
        None
    }
}

pub fn is_win11() -> bool {
    if let Some(info) = os_version_info() {
        info.dwBuildNumber >= 22000
    } else {
        false
    }
}
