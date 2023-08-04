mod check_error;
mod single_instance;
mod window;
mod windows_icon;

pub use check_error::*;
pub use single_instance::*;
pub use window::*;
pub use windows_icon::get_module_icon_ex;

pub fn to_wstring(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect::<Vec<u16>>()
}
