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
        Foundation::{COLORREF, HWND, WPARAM},
        Graphics::Gdi::{
            CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetObjectW, GetPixel, ReleaseDC,
            SelectObject, BITMAP, HGDIOBJ,
        },
        Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
        UI::{
            Controls::IImageList,
            Shell::{SHGetFileInfoW, SHGetImageList, SHFILEINFOW, SHGFI_SYSICONINDEX},
            WindowsAndMessaging::{
                CopyIcon, CreateIconFromResourceEx, DestroyIcon, GetIconInfo, LoadIconW,
                LoadImageW, SendMessageW, GCL_HICON, HICON, ICONINFO, ICON_BIG, IDI_APPLICATION,
                IMAGE_ICON, LR_DEFAULTCOLOR, LR_DEFAULTSIZE, LR_LOADFROMFILE, WM_GETICON,
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

    if let Some(icon) = get_pwa_icon_from_lnk(module_path) {
        return icon;
    }

    if let Some(icon) = get_browser_profile_icon(module_path) {
        return icon;
    }

    if module_path.starts_with("C:\\Program Files\\WindowsApps") {
        if let Some(icon) =
            get_appx_logo_path(module_path).and_then(|image_path| load_image_as_hicon(&image_path))
        {
            return icon;
        }
    }

    let base_path = module_path.split("::").next().unwrap_or(module_path);
    get_exe_icon(base_path)
        .or_else(|| get_window_icon(hwnd))
        .unwrap_or_else(fallback_icon)
}

fn get_appx_logo_path(module_path: &str) -> Option<PathBuf> {
    let module_path = PathBuf::from(module_path);
    let executable = module_path.file_name()?.to_string_lossy();
    let module_dir = module_path.parent()?;
    let logo_value = read_appx_logo_value(module_dir, Some(&executable))?;
    resolve_appx_logo_path(module_dir, &logo_value)
}

fn get_appx_logo_from_dir(package_dir: &Path) -> Option<PathBuf> {
    let logo_value = read_appx_logo_value(package_dir, None)?;
    resolve_appx_logo_path(package_dir, &logo_value)
}

fn read_appx_logo_value(manifest_dir: &Path, executable: Option<&str>) -> Option<String> {
    let manifest_path = manifest_dir.join("AppxManifest.xml");
    let manifest_file = File::open(manifest_path).ok()?;
    let manifest_file = BufReader::new(manifest_file);
    let reader = EventReader::new(manifest_file);
    let mut logo_value = None;
    let mut matched = executable.is_none();
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
                    if let Some(exe) = executable {
                        matched = attributes
                            .iter()
                            .any(|v| v.name.local_name == "Executable" && v.value == exe);
                    }
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
            Err(_) => break,
            _ => {}
        }
    }
    logo_value
}

fn resolve_appx_logo_path(base_dir: &Path, logo_value: &str) -> Option<PathBuf> {
    let logo_path = base_dir.join(logo_value);
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
        unsafe { CreateIconFromResourceEx(&buffer, true, 0x30000, 100, 100, LR_DEFAULTCOLOR) }.ok()
    }
}

fn fallback_icon() -> HICON {
    let icon = unsafe { LoadIconW(None, IDI_APPLICATION) }.unwrap_or_default();
    unsafe { CopyIcon(icon) }.unwrap_or_default()
}

pub fn get_window_icon(hwnd: HWND) -> Option<HICON> {
    let ret = unsafe { SendMessageW(hwnd, WM_GETICON, Some(WPARAM(ICON_BIG as _)), None) };
    if ret.0 != 0 {
        return unsafe { CopyIcon(HICON(ret.0 as _)) }.ok();
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

fn get_browser_profile_icon(module_path: &str) -> Option<HICON> {
    let parts: Vec<&str> = module_path.split("::").collect();
    if parts.len() != 2 {
        return None;
    }
    let exe_path = parts[0];
    let profile = parts[1];

    let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
    let (user_data_dir, icon_file) = if exe_path.to_lowercase().contains("chrome.exe") {
        (
            PathBuf::from(&local_app_data).join(r"Google\Chrome\User Data"),
            "Google Profile.ico",
        )
    } else if exe_path.to_lowercase().contains("msedge.exe") {
        (
            PathBuf::from(&local_app_data).join(r"Microsoft\Edge\User Data"),
            "Edge Profile.ico",
        )
    } else {
        return None;
    };

    let profile_dir = super::window::pwa_map_profile_dir(profile);
    let icon_path = user_data_dir.join(&profile_dir).join(icon_file);
    load_image_as_hicon(&icon_path)
}

fn get_pwa_icon_from_lnk(module_path: &str) -> Option<HICON> {
    let parts: Vec<&str> = module_path.split("::").collect();
    if parts.len() != 3 {
        return None;
    }
    let exe_path = parts[0];
    let typ = parts[1];
    let app_id = parts[2];

    if typ == "appx" {
        let package_dir = super::window::find_appx_pkg_dir(app_id)?;
        let logo_path = get_appx_logo_from_dir(&PathBuf::from(package_dir))?;
        load_image_as_hicon(&logo_path)
    } else {
        let user_data_dir = super::window::get_default_user_data_dir(exe_path)?;
        let lnk_path = super::window::pwa_find_lnk_path(&user_data_dir, typ, app_id)?;
        get_exe_icon(&lnk_path.to_string_lossy())
    }
}

fn get_exe_icon(module_path: &str) -> Option<HICON> {
    unsafe {
        let r: ::windows::core::Result<IImageList> = SHGetImageList(0x04);
        match r {
            ::windows::core::Result::Ok(list) => {
                if let Some(info) = get_shfileinfo(module_path) {
                    let r = list.GetIcon(info.iIcon, 1u32);
                    match r {
                        Ok(hicon) => {
                            let size = get_icon_size(hicon);
                            match size {
                                Some((x, y)) if x >= 64 && y >= 64 => Some(hicon),
                                _ => {
                                    let _ = DestroyIcon(hicon);
                                    None
                                }
                            }
                        }
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
        for _ in 0..3 {
            // sporadically this method returns 0
            let fff: usize = SHGetFileInfoW(
                PCWSTR::from_raw(p_path.as_mut_ptr()),
                FILE_ATTRIBUTE_NORMAL,
                Some(&mut file_info),
                mem::size_of_val(&file_info) as u32,
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

fn get_icon_size(hicon: HICON) -> Option<(i32, i32)> {
    unsafe {
        let mut icon_info: ICONINFO = std::mem::zeroed();
        if GetIconInfo(hicon, &mut icon_info).is_err() {
            return None;
        }

        let mut bmp = BITMAP::default();
        if 0 == GetObjectW(
            icon_info.hbmColor.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bmp as *mut _ as *mut _),
        ) {
            let _ = DeleteObject(icon_info.hbmColor.into());
            let _ = DeleteObject(icon_info.hbmMask.into());
            return None;
        }

        let (width, height) = (bmp.bmWidth, bmp.bmHeight);
        let hdc = GetDC(None);
        let hmemdc = CreateCompatibleDC(Some(hdc));
        let old_bitmap = SelectObject(hmemdc, HGDIOBJ(icon_info.hbmColor.0 as _));

        let (mut min_x, mut min_y, mut max_x, mut max_y) = (width, height, 0, 0);

        for y in 0..height {
            for x in 0..width {
                let pixel = GetPixel(hmemdc, x, y);
                if pixel != COLORREF(0) {
                    if x < min_x {
                        min_x = x;
                    }
                    if y < min_y {
                        min_y = y;
                    }
                    if x > max_x {
                        max_x = x;
                    }
                    if y > max_y {
                        max_y = y;
                    }
                }
            }
        }

        SelectObject(hmemdc, old_bitmap);
        let _ = DeleteDC(hmemdc);
        ReleaseDC(None, hdc);
        let _ = DeleteObject(icon_info.hbmColor.into());
        let _ = DeleteObject(icon_info.hbmMask.into());

        Some((max_x - min_x + 1, max_y - min_y + 1))
    }
}
