use std::{mem, time};

use windows::{
    core::PCWSTR,
    Win32::{
        Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
        UI::{
            Controls::IImageList,
            Shell::{SHGetFileInfoW, SHGetImageList, SHFILEINFOW, SHGFI_SYSICONINDEX},
            WindowsAndMessaging::HICON,
        },
    },
};

fn get_module_icon_ex0(ext: &str) -> Option<SHFILEINFOW> {
    unsafe {
        let mut p_path: Vec<u16> = ext.encode_utf16().chain(std::iter::once(0)).collect();
        let mut file_info = SHFILEINFOW::default();
        let file_info_size = mem::size_of_val(&file_info) as u32;
        for _ in 0..3 {
            // sporadically this method returns 0
            let fff: usize = SHGetFileInfoW(
                PCWSTR::from_raw(p_path.as_mut_ptr()),
                FILE_ATTRIBUTE_NORMAL,
                Some(&mut file_info),
                file_info_size,
                SHGFI_SYSICONINDEX,
            );
            if fff != 0 {
                return Some(file_info);
            } else {
                let millis = time::Duration::from_millis(30);
                std::thread::sleep(millis);
            }
        }
        None
    }
}

pub fn get_module_icon_ex(ext: &str) -> Option<HICON> {
    unsafe {
        let r: ::windows::core::Result<IImageList> = SHGetImageList(0x04);
        match r {
            ::windows::core::Result::Ok(list) => {
                if let Some(icon) = get_module_icon_ex0(ext) {
                    let r = list.GetIcon(icon.iIcon, 1u32);
                    match r {
                        Ok(v) => Some(v),
                        Err(_) => None,
                    }
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }
}
