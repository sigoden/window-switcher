use crate::app::SwitchAppsState;
use crate::utils::RegKey;
use anyhow::Result;
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, RECT};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush, DeleteDC,
    DeleteObject, EndPaint, FillRect, SelectObject, SetStretchBltMode, StretchBlt, HALFTONE,
    HBITMAP, HBRUSH, HDC, PAINTSTRUCT, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{DrawIconEx, DI_NORMAL};
use windows::Win32::{Foundation::HWND, Graphics::Gdi::GetDC};

// window background color in dark theme
pub const BG_DARK_COLOR: COLORREF = COLORREF(0x3b3b3b);
// selected icon box color in dark theme
pub const FG_DARK_COLOR: COLORREF = COLORREF(0x4c4c4c);
// window background color in light theme
pub const BG_LIGHT_COLOR: COLORREF = COLORREF(0xf2f2f2);
// selected icon box color in light theme
pub const FG_LIGHT_COLOR: COLORREF = COLORREF(0xe0e0e0);
// minimum icon size
pub const ICON_SIZE: i32 = 64;
// window padding
pub const WINDOW_BORDER_SIZE: i32 = 10;
// icon border
pub const ICON_BORDER_SIZE: i32 = 4;

// GDI Antialiasing Painter
pub struct GdiAAPainter {
    // memory
    mem_hdc: HDC,
    mem_map: HBITMAP,
    // scaled
    scaled_hdc: HDC,
    scaled_map: HBITMAP,
    // windows handle
    hwnd: HWND,
    // content size
    width: i32,
    height: i32,
    size: i32,
    // scale
    scale: i32,
    // color
    fg_color: COLORREF,
    bg_color: COLORREF,
}

impl GdiAAPainter {
    /// Creates a new [GdiAAPainter] instance.
    ///
    /// The `scale` must be a multiple of 2, for example 2, 4, 6, 8, 12 ...
    pub fn new(hwnd: HWND, scale: i32) -> Self {
        let light_theme = match is_light_theme() {
            Ok(v) => v,
            Err(_) => {
                warn!("Fail to get system theme");
                false
            }
        };
        let (fg_color, bg_color) = match light_theme {
            true => (FG_LIGHT_COLOR, BG_LIGHT_COLOR),
            false => (FG_DARK_COLOR, BG_DARK_COLOR),
        };
        GdiAAPainter {
            mem_hdc: Default::default(),
            mem_map: Default::default(),
            scaled_hdc: Default::default(),
            scaled_map: Default::default(),
            hwnd,
            width: 0,
            height: 0,
            size: 0,
            scale,
            fg_color,
            bg_color,
        }
    }

    /// Initial this painter.
    ///
    /// Returns (icon_size, width, height)
    pub fn init(&mut self, monitor_width: i32, num_apps: i32) -> (i32, i32, i32) {
        let icon_size = ((monitor_width - 2 * WINDOW_BORDER_SIZE) / num_apps
            - ICON_BORDER_SIZE * 2)
            .min(ICON_SIZE);

        let item_size = icon_size + ICON_BORDER_SIZE * 2;
        let width = item_size * num_apps + WINDOW_BORDER_SIZE * 2;
        let height = item_size + WINDOW_BORDER_SIZE * 2;
        let size = width * height;
        if size == self.size {
            return (icon_size, width, height);
        }

        unsafe {
            self.width = width;
            self.height = height;
            self.size = size;

            let _ = DeleteDC(self.mem_hdc);
            let _ = DeleteObject(self.mem_map);
            let _ = DeleteDC(self.scaled_hdc);
            let _ = DeleteObject(self.scaled_map);

            let hdc = GetDC(self.hwnd);
            let mem_dc = CreateCompatibleDC(hdc);
            let mem_map = CreateCompatibleBitmap(hdc, width, height);
            SelectObject(mem_dc, mem_map);

            let brush = CreateSolidBrush(self.fg_color);
            let rect = RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            };
            FillRect(mem_dc, &rect as _, brush);

            let scaled_dc = CreateCompatibleDC(hdc);
            let scaled_map = CreateCompatibleBitmap(hdc, width * self.scale, height * self.scale);
            SelectObject(scaled_dc, scaled_map);
            let rect = RECT {
                left: 0,
                top: 0,
                right: width * self.scale,
                bottom: height * self.scale,
            };
            FillRect(scaled_dc, &rect as _, brush);

            self.mem_hdc = mem_dc;
            self.mem_map = mem_map;
            self.scaled_hdc = scaled_dc;
            self.scaled_map = scaled_map;
        }

        (icon_size, width, height)
    }

    /// Draw state onto hdc in memory
    pub fn paint(&mut self, state: &SwitchAppsState) {
        self.paint0(state);
        unsafe {
            SetStretchBltMode(self.mem_hdc, HALFTONE);
            let _ = StretchBlt(
                self.mem_hdc,
                0,
                0,
                self.width,
                self.height,
                self.scaled_hdc,
                0,
                0,
                self.width * self.scale,
                self.height * self.scale,
                SRCCOPY,
            );
        }
    }

    pub fn display(&mut self) {
        unsafe {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(self.hwnd, &mut ps);
            let _ = BitBlt(
                hdc,
                0,
                0,
                self.width,
                self.height,
                self.mem_hdc,
                0,
                0,
                SRCCOPY,
            );
            let _ = EndPaint(self.hwnd, &ps);
        }
    }

    fn paint0(&mut self, state: &SwitchAppsState) {
        unsafe {
            // draw background
            let rect = RECT {
                left: 0,
                top: 0,
                right: self.width * self.scale,
                bottom: self.width * self.scale,
            };
            FillRect(self.scaled_hdc, &rect as _, CreateSolidBrush(self.fg_color));

            let cy = (WINDOW_BORDER_SIZE + ICON_BORDER_SIZE) * self.scale;
            let brush_icon = HBRUSH::default();
            let item_size = (state.icon_size + ICON_BORDER_SIZE * 2) * self.scale;

            for (i, (icon, _)) in state.apps.iter().enumerate() {
                // draw the box for selected icon
                if i == state.index {
                    let left = item_size * (i as i32) + WINDOW_BORDER_SIZE * self.scale;
                    let top = WINDOW_BORDER_SIZE * self.scale;
                    let right = left + item_size;
                    let bottom = top + item_size;
                    let rect = RECT {
                        left,
                        top,
                        right,
                        bottom,
                    };
                    FillRect(self.scaled_hdc, &rect as _, CreateSolidBrush(self.bg_color));
                }

                let cx = cy + item_size * (i as i32);
                let _ = DrawIconEx(
                    self.scaled_hdc,
                    cx,
                    cy,
                    *icon,
                    state.icon_size * self.scale,
                    state.icon_size * self.scale,
                    0,
                    brush_icon,
                    DI_NORMAL,
                );
            }
        }
    }
}

fn is_light_theme() -> Result<bool> {
    let reg_key = RegKey::new_hkcu(
        w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
        w!("SystemUsesLightTheme"),
    )?;
    let value = reg_key.get_int()?;
    Ok(value == 1)
}
