use super::to_wstring;

use std::{
    fs::File,
    io::{BufReader, Read},
    mem,
    path::{Path, PathBuf},
    time,
};

use indexmap::IndexMap;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HWND, TRUE, WPARAM},
        Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
        UI::{
            Controls::IImageList,
            Shell::{SHGetFileInfoW, SHGetImageList, SHFILEINFOW, SHGFI_SYSICONINDEX},
            WindowsAndMessaging::{
                CopyIcon, CreateIconFromResourceEx, LoadIconW, LoadImageW, SendMessageW, GCL_HICON,
                HICON, ICON_BIG, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTCOLOR, LR_DEFAULTSIZE,
                LR_LOADFROMFILE, WM_GETICON,
            },
        },
    },
};
use xml::reader::XmlEvent;
use xml::EventReader;

pub fn get_app_icon(
    override_icons: &IndexMap<String, String>,
    module_path: &str,
    hwnd: HWND,
) -> HICON {
    let module_path_lc = module_path.to_lowercase();
    if let Some((_, v)) = override_icons
        .iter()
        .find(|(k, _)| module_path_lc.contains(*k))
    {
        let mut override_path = PathBuf::from(v);
        if !override_path.is_absolute() {
            if let Some(module_dir) = Path::new(module_path).parent() {
                override_path = module_dir.join(override_path);
            }
        }
        if let Some(icon) = load_image_as_hicon(override_path) {
            return icon;
        }
    }

    if module_path.starts_with("C:\\Program Files\\WindowsApps") {
        if let Some(icon) =
            get_appx_logo_path(module_path).and_then(|image_path| load_image_as_hicon(&image_path))
        {
            return icon;
        }
    }

    get_exe_icon(module_path)
        .or_else(|| get_window_icon(hwnd))
        .unwrap_or_else(fallback_icon)
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
    for size in ["targetsize-256", "targetsize-128", "scale-200", "scale-100"] {
        let logo_path = PathBuf::from(format!("{prefix}.{size}{extension}"));
        if logo_path.exists() {
            return Some(logo_path);
        }
    }
    None
}

pub fn load_image_as_hicon<T: AsRef<Path>>(image_path: T) -> Option<HICON> {
    let image_path = image_path.as_ref();
    if !image_path.exists() {
        return None;
    }
    if let Some("ico") = image_path.extension().and_then(|v| v.to_str()) {
        let icon_path = to_wstring(image_path.to_string_lossy().as_ref());
        unsafe {
            LoadImageW(
                None,
                PCWSTR(icon_path.as_ptr()),
                IMAGE_ICON,
                256,
                256,
                LR_LOADFROMFILE | LR_DEFAULTSIZE,
            )
        }
        .ok()
        .map(|v| HICON(v.0))
    } else {
        let mut logo_file = File::open(image_path).ok()?;
        let mut buffer = vec![];
        logo_file.read_to_end(&mut buffer).ok()?;
        unsafe { CreateIconFromResourceEx(&buffer, TRUE, 0x30000, 100, 100, LR_DEFAULTCOLOR) }.ok()
    }
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
