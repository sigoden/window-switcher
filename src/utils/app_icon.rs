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
        Foundation::{HWND, LPARAM, WPARAM},
        Graphics::Gdi::{
            CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC,
            SelectObject, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HBITMAP, HDC,
            HGDIOBJ, RGBQUAD,
        },
        Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
        UI::{
            Controls::IImageList,
            Shell::{SHGetFileInfoW, SHGetImageList, SHFILEINFOW, SHGFI_SYSICONINDEX},
            WindowsAndMessaging::{
                CopyIcon, CreateIconFromResourceEx, DestroyIcon, GetIconInfo, LoadIconW,
                LoadImageW, SendMessageTimeoutW, GCLP_HICONSM, GCL_HICON, HICON, ICONINFO,
                ICON_BIG, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTCOLOR, LR_DEFAULTSIZE,
                LR_LOADFROMFILE, SMTO_ABORTIFHUNG, WM_GETICON,
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
    let mut result: usize = 0;
    let ret = unsafe {
        SendMessageTimeoutW(
            hwnd,
            WM_GETICON,
            WPARAM(ICON_BIG as _),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            250,
            Some(&mut result),
        )
    };
    if ret.0 != 0 && result != 0 {
        return unsafe { CopyIcon(HICON(result as _)) }.ok();
    }
    #[cfg(target_arch = "x86")]
    let ret = unsafe { windows::Win32::UI::WindowsAndMessaging::GetClassLongW(hwnd, GCL_HICON) };
    #[cfg(not(target_arch = "x86"))]
    let ret = unsafe { windows::Win32::UI::WindowsAndMessaging::GetClassLongPtrW(hwnd, GCL_HICON) };
    if ret != 0 {
        return unsafe { CopyIcon(HICON(ret as _)) }.ok();
    }
    #[cfg(target_arch = "x86")]
    let ret = unsafe { windows::Win32::UI::WindowsAndMessaging::GetClassLongW(hwnd, GCLP_HICONSM) };
    #[cfg(not(target_arch = "x86"))]
    let ret =
        unsafe { windows::Win32::UI::WindowsAndMessaging::GetClassLongPtrW(hwnd, GCLP_HICONSM) };
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

const SHIL_JUMBO: i32 = 0x04;
const SHIL_EXTRALARGE: i32 = 0x02;
const SHIL_LARGE: i32 = 0x00;

fn get_exe_icon(module_path: &str) -> Option<HICON> {
    let info = get_shfileinfo(module_path)?;
    for shil in [SHIL_JUMBO, SHIL_EXTRALARGE, SHIL_LARGE] {
        unsafe {
            let list = SHGetImageList::<IImageList>(shil).ok()?;
            let hicon = list.GetIcon(info.iIcon, 1u32).ok()?;
            if is_valid_icon(hicon) {
                return Some(hicon);
            } else {
                let _ = DestroyIcon(hicon);
            }
        }
    }
    None
}

fn get_shfileinfo(module_path: &str) -> Option<SHFILEINFOW> {
    unsafe {
        let mut p_path: Vec<u16> = module_path
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let mut file_info = SHFILEINFOW::default();
        // Retry up to 3 times because SHGetFileInfoW can transiently fail
        // (e.g. shell not fully initialized, file system contention). A simple
        // short sleep + retry handles these spurious failures robustly.
        for _ in 0..3 {
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

/// Returns `false` for icons whose content is squeezed into the top-left corner
/// while the rest is stretched/padded garbage — e.g. the icon of `hh.exe`
/// (Windows's help viewer, reused by AutoHotKey, etc. to display their *.chm help).
/// These look ugly in the switcher and are better replaced with a fallback icon.
pub fn is_valid_icon(hicon: HICON) -> bool {
    let Some(bounds) = get_icon_bounds(hicon) else {
        return false;
    };
    !is_topleft_icon(&bounds)
}

struct IconBounds {
    pub canvas_width: i32,
    pub canvas_height: i32,
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

fn is_topleft_icon(bounds: &IconBounds) -> bool {
    let bbox_width = bounds.max_x - bounds.min_x + 1;
    let bbox_height = bounds.max_y - bounds.min_y + 1;

    let bbox_area = bbox_width * bbox_height;
    let canvas_area = bounds.canvas_width * bounds.canvas_height;

    let bbox_ratio = bbox_area as f32 / canvas_area as f32;

    let center_x = (bounds.min_x + bounds.max_x) as f32 / 2.0 / bounds.canvas_width as f32;
    let center_y = (bounds.min_y + bounds.max_y) as f32 / 2.0 / bounds.canvas_height as f32;

    let small_content = bbox_ratio < 0.25;
    let top_left = center_x < 0.30 && center_y < 0.30;

    small_content && top_left
}

struct HdcGuard(HDC);
impl Drop for HdcGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteDC(self.0);
        }
    }
}

struct ScreenDcGuard(HDC);
impl Drop for ScreenDcGuard {
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(None, self.0);
        }
    }
}

struct BitmapGuard(HBITMAP);
impl Drop for BitmapGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(self.0 .0 as _));
        }
    }
}

fn get_icon_bounds(hicon: HICON) -> Option<IconBounds> {
    unsafe {
        let mut icon_info: ICONINFO = std::mem::zeroed();
        if GetIconInfo(hicon, &mut icon_info).is_err() {
            return None;
        }
        let _color_guard = BitmapGuard(icon_info.hbmColor);
        let _mask_guard = BitmapGuard(icon_info.hbmMask);

        let mut bmp = BITMAP::default();
        if GetObjectW(
            icon_info.hbmColor.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bmp as *mut _ as *mut _),
        ) == 0
        {
            return None;
        }

        let width = bmp.bmWidth;
        let height = bmp.bmHeight;
        if width <= 0 || height <= 0 {
            return None;
        }

        let screen_dc = GetDC(None);
        let _screen_guard = ScreenDcGuard(screen_dc);

        let mem_dc = CreateCompatibleDC(Some(screen_dc));
        let _dc_guard = HdcGuard(mem_dc);

        let old_bmp = SelectObject(mem_dc, HGDIOBJ(icon_info.hbmColor.0 as _));

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }; 1],
        };

        let buf_size = (width * height * 4) as usize;
        let mut pixels: Vec<u8> = vec![0; buf_size];

        if 0 == GetDIBits(
            mem_dc,
            icon_info.hbmColor,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        ) {
            SelectObject(mem_dc, old_bmp);
            return None;
        }

        let mut min_x = width;
        let mut min_y = height;
        let mut max_x: i32 = -1;
        let mut max_y: i32 = -1;
        let mut has_semi_transparent = false;

        // Pass 1: top/bottom bounds + alpha detection in one scan
        let rows = pixels.chunks_exact(width as usize * 4);

        for (y, row) in rows.clone().enumerate() {
            let mut row_has_visible = false;
            for c in row.chunks_exact(4) {
                let a = c[3];
                if a != 0 {
                    row_has_visible = true;
                    if a < 255 {
                        has_semi_transparent = true;
                    }
                }
            }
            if row_has_visible {
                min_y = y as i32;
                break;
            }
        }

        if min_y < height {
            for (y, row) in rows.clone().rev().enumerate() {
                let actual_y = (height as usize - 1) - y;
                let mut row_has_visible = false;
                for c in row.chunks_exact(4) {
                    let a = c[3];
                    if a != 0 {
                        row_has_visible = true;
                        if a < 255 {
                            has_semi_transparent = true;
                        }
                    }
                }
                if row_has_visible {
                    max_y = actual_y as i32;
                    break;
                }
            }
        }

        // Pass 2: left/right bounds within [min_y, max_y]
        if max_y >= 0 {
            let stride = width as usize * 4;

            'left: for x in 0..width {
                for y in min_y..=max_y {
                    if pixels[(y as usize * stride + x as usize * 4) + 3] != 0 {
                        min_x = x;
                        break 'left;
                    }
                }
            }

            'right: for x in (0..width).rev() {
                for y in min_y..=max_y {
                    if pixels[(y as usize * stride + x as usize * 4) + 3] != 0 {
                        max_x = x;
                        break 'right;
                    }
                }
            }
        }

        // Fallback to mask when no alpha channel and no visible pixels found
        if !has_semi_transparent && max_x < 0 {
            SelectObject(mem_dc, HGDIOBJ(icon_info.hbmMask.0 as _));

            if 0 != GetDIBits(
                mem_dc,
                icon_info.hbmMask,
                0,
                height as u32,
                Some(pixels.as_mut_ptr() as *mut _),
                &mut bmi,
                DIB_RGB_COLORS,
            ) {
                let mask_rows = pixels.chunks_exact(width as usize * 4);

                min_y = height;
                max_y = -1i32;

                for (y, row) in mask_rows.clone().enumerate() {
                    if row
                        .chunks_exact(4)
                        .any(|c| c[0] == 0 && c[1] == 0 && c[2] == 0)
                    {
                        min_y = y as i32;
                        break;
                    }
                }

                if min_y < height {
                    for (y, row) in mask_rows.clone().rev().enumerate() {
                        let actual_y = (height as usize - 1) - y;
                        if row
                            .chunks_exact(4)
                            .any(|c| c[0] == 0 && c[1] == 0 && c[2] == 0)
                        {
                            max_y = actual_y as i32;
                            break;
                        }
                    }
                }

                if max_y >= 0 {
                    let stride = width as usize * 4;

                    'mleft: for x in 0..width {
                        for y in min_y..=max_y {
                            let base = y as usize * stride + x as usize * 4;
                            if pixels[base] == 0 && pixels[base + 1] == 0 && pixels[base + 2] == 0 {
                                min_x = x;
                                break 'mleft;
                            }
                        }
                    }

                    'mright: for x in (0..width).rev() {
                        for y in min_y..=max_y {
                            let base = y as usize * stride + x as usize * 4;
                            if pixels[base] == 0 && pixels[base + 1] == 0 && pixels[base + 2] == 0 {
                                max_x = x;
                                break 'mright;
                            }
                        }
                    }
                }
            }

            SelectObject(mem_dc, old_bmp);
        } else {
            SelectObject(mem_dc, old_bmp);
        }

        if max_x < 0 {
            return None;
        }

        Some(IconBounds {
            canvas_width: width,
            canvas_height: height,
            min_x,
            min_y,
            max_x,
            max_y,
        })
    }
}
