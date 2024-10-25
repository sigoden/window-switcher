use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
    mem,
    path::{Path, PathBuf},
    time,
};

use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HWND, TRUE, WPARAM},
        Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
        UI::{
            Controls::IImageList,
            Shell::{SHGetFileInfoW, SHGetImageList, SHFILEINFOW, SHGFI_SYSICONINDEX},
            WindowsAndMessaging::{
                CopyIcon, CreateIconFromResourceEx, LoadIconW, SendMessageW, GCL_HICON, HICON,
                ICON_BIG, IDI_APPLICATION, LR_DEFAULTCOLOR, WM_GETICON,
            },
        },
    },
};
use xml::reader::XmlEvent;
use xml::EventReader;

pub fn get_app_icon(
    cached_icons: &mut HashMap<String, HICON>,
    module_path: &str,
    hwnd: HWND,
) -> HICON {
    if let Some(icon) = cached_icons.get(module_path) {
        return *icon;
    }

    if module_path.starts_with("C:\\Program Files\\WindowsApps") {
        let icon = get_appx_logo_path(module_path)
            .and_then(|image_path| load_image_as_hicon(&image_path))
            .unwrap_or_else(fallback_icon);
        cached_icons.insert(module_path.to_string(), icon);
        return icon;
    }
    let icon = get_exe_icon(module_path)
        .or_else(|| get_window_icon(hwnd))
        .unwrap_or_else(fallback_icon);
    cached_icons.insert(module_path.to_string(), icon);
    icon
}

fn get_appx_logo_path(module_path: &str) -> Option<PathBuf> {
    let module_path = PathBuf::from(module_path);
    let executable = module_path.file_name()?.to_string_lossy();
    let module_dir = module_path.parent()?;
    let manifest_path = module_dir.join("AppxManifest.xml");
    let manifest_file = File::open(manifest_path).ok()?;
    let manifest_file = BufReader::new(manifest_file); // Buffering is important for performance
    let reader = EventReader::new(manifest_file);
    let mut logo_value = None;
    let mut matched = false;
    let mut paths = vec![];
    let mut depth = 0;
    for e in reader {
        match e {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => {
                if paths.len() == depth {
                    paths.push(name.local_name.clone())
                }
                let xpath = paths.join("/");
                if xpath == "Package/Applications/Application" {
                    matched = attributes
                        .iter()
                        .any(|v| v.name.local_name == "Executable" && v.value == executable);
                } else if xpath == "Package/Applications/Application/VisualElements" && matched {
                    if let Some(value) = attributes
                        .iter()
                        .find(|v| {
                            ["Square44x44Logo", "Square30x30Logo", "SmallLogo"]
                                .contains(&v.name.local_name.as_str())
                        })
                        .map(|v| v.value.clone())
                    {
                        logo_value = Some(value);
                        break;
                    }
                }
                depth += 1;
            }
            Ok(XmlEvent::EndElement { .. }) => {
                if paths.len() == depth {
                    paths.pop();
                }
                depth -= 1;
            }
            Err(_) => {
                break;
            }
            _ => {}
        }
    }
    let logo_path = module_dir.join(logo_value?);
    let extension = format!(".{}", logo_path.extension()?.to_string_lossy());
    let logo_path = logo_path.display().to_string();
    let prefix = &logo_path[0..(logo_path.len() - extension.len())];
    for size in [
        "targetsize-256",
        "targetsize-128",
        "targetsize-72",
        "targetsize-36",
        "scale-200",
        "scale-100",
    ] {
        let logo_path = PathBuf::from(format!("{prefix}.{size}{extension}"));
        if logo_path.exists() {
            return Some(logo_path);
        }
    }
    None
}

pub fn load_image_as_hicon<T: AsRef<Path>>(image_path: T) -> Option<HICON> {
    let mut logo_file = File::open(image_path.as_ref()).ok()?;
    let mut buffer = vec![];
    logo_file.read_to_end(&mut buffer).ok()?;
    unsafe { CreateIconFromResourceEx(&buffer, TRUE, 0x30000, 100, 100, LR_DEFAULTCOLOR) }.ok()
}

fn fallback_icon() -> HICON {
    unsafe { LoadIconW(None, IDI_APPLICATION) }.unwrap_or_default()
}

pub fn get_window_icon(hwnd: HWND) -> Option<HICON> {
    let ret = unsafe { SendMessageW(hwnd, WM_GETICON, WPARAM(ICON_BIG as _), None) };
    if ret.0 != 0 {
        return Some(HICON(ret.0 as _));
    }
    #[cfg(target_arch = "x86")]
    let ret = unsafe { windows::Win32::UI::WindowsAndMessaging::GetClassLongW(hwnd, GCL_HICON) };
    #[cfg(not(target_arch = "x86"))]
    let ret = unsafe { windows::Win32::UI::WindowsAndMessaging::GetClassLongPtrW(hwnd, GCL_HICON) };
    if ret != 0 {
        return unsafe { CopyIcon(HICON(ret as _)) }.ok();
    }
    None
}

fn get_exe_icon(module_path: &str) -> Option<HICON> {
    unsafe {
        let r: ::windows::core::Result<IImageList> = SHGetImageList(0x04);
        match r {
            ::windows::core::Result::Ok(list) => {
                if let Some(icon) = get_shfileinfo(module_path) {
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

fn get_shfileinfo(module_path: &str) -> Option<SHFILEINFOW> {
    unsafe {
        let mut p_path: Vec<u16> = module_path
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
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
