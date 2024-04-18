mod admin;
mod check_error;
mod regedit;
mod scheduled_task;
mod single_instance;
mod window;
mod windows_icon;

pub use admin::*;
pub use check_error::*;
pub use regedit::*;
pub use scheduled_task::*;
pub use single_instance::*;
pub use window::*;
pub use windows_icon::get_module_icon_ex;

pub fn to_wstring(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect::<Vec<u16>>()
}
