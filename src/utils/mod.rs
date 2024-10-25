mod admin;
mod app_icon;
mod check_error;
mod handle_wrapper;
mod regedit;
mod scheduled_task;
mod single_instance;
mod window;
mod windows_theme;
mod windows_version;

pub use admin::*;
pub use app_icon::*;
pub use check_error::*;
pub use handle_wrapper::*;
pub use regedit::*;
pub use scheduled_task::*;
pub use single_instance::*;
pub use window::*;
pub use windows_theme::*;
pub use windows_version::*;

pub fn to_wstring(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect::<Vec<u16>>()
}
