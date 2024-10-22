use crate::app::SwitchAppsState;
use crate::utils::{check_error, get_moinitor_rect, RegKey};
use anyhow::{Context, Result};
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HMODULE, POINT, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, ReleaseDC, SelectObject,
    AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, HDC,
};
use windows::Win32::Graphics::GdiPlus::{
    FillModeAlternate, GdipAddPathArc, GdipClosePathFigure, GdipCreateBitmapFromHICON,
    GdipCreateFromHDC, GdipCreatePath, GdipCreatePen1, GdipDeleteBrush, GdipDeleteGraphics,
    GdipDeletePath, GdipDeletePen, GdipDrawImageRect, GdipFillPath, GdipGetPenBrushFill,
    GdipSetSmoothingMode, GdiplusShutdown, GdiplusStartup, GdiplusStartupInput, GpBitmap, GpBrush,
    GpGraphics, GpImage, GpPath, GpPen, SmoothingModeAntiAlias, Unit,
};
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, LoadCursorW, SetCursor, ShowWindow, UpdateLayeredWindow, IDC_ARROW, SW_HIDE,
    SW_SHOW, ULW_ALPHA,
};
use windows::Win32::{Foundation::HWND, Graphics::Gdi::GetDC};

pub const BG_DARK_COLOR: u32 = 0x3b3b3b;
pub const FG_DARK_COLOR: u32 = 0x4c4c4c;
pub const BG_LIGHT_COLOR: u32 = 0xf2f2f2;
pub const FG_LIGHT_COLOR: u32 = 0xe0e0e0;
pub const ALPHA_MASK: u32 = 0xff000000;
// maximum icon size
pub const ICON_SIZE: i32 = 64;
// window padding
pub const WINDOW_BORDER_SIZE: i32 = 10;
// icon border
pub const ICON_BORDER_SIZE: i32 = 4;

// GDI Antialiasing Painter
pub struct GdiAAPainter {
    token: usize,
    hwnd: HWND,
    hdc_screen: HDC,
    show: bool,

    x: i32,
    y: i32,
    width: i32,
    height: i32,
    corner_radius: i32,
    icon_size: i32,

    fg_color: u32,
    bg_color: u32,
}

impl GdiAAPainter {
    pub fn new(hwnd: HWND) -> Result<Self> {
        let startup_input = GdiplusStartupInput {
            GdiplusVersion: 1,
            ..Default::default()
        };
        let mut token: usize = 0;
        check_error(|| unsafe { GdiplusStartup(&mut token, &startup_input, std::ptr::null_mut()) })
            .context("Failed to initialize GDI+")?;

        let hdc_screen = unsafe { GetDC(hwnd) };

        let light_theme = match is_light_theme() {
            Ok(v) => v,
            Err(_) => {
                warn!("Fail to get system theme");
                false
            }
        };

        let (fg_color, bg_color) = theme_color(light_theme);

        Ok(Self {
            token,
            hwnd,
            hdc_screen,

            show: false,

            x: 0,
            y: 0,
            width: 0,
            height: 0,
            corner_radius: 0,
            icon_size: 0,

            fg_color,
            bg_color,
        })
    }

    pub fn icon_size(&self) -> i32 {
        self.icon_size
    }

    pub fn prepare(&mut self, num_apps: i32) {
        let monitor_rect = get_moinitor_rect();
        let monitor_width = monitor_rect.right - monitor_rect.left;
        let monitor_height = monitor_rect.bottom - monitor_rect.top;

        let icon_size = ((monitor_width - 2 * WINDOW_BORDER_SIZE) / num_apps
            - ICON_BORDER_SIZE * 2)
            .min(ICON_SIZE);

        let item_size = icon_size + ICON_BORDER_SIZE * 2;
        let width = item_size * num_apps + WINDOW_BORDER_SIZE * 2;
        let height = item_size + WINDOW_BORDER_SIZE * 2;
        self.x = monitor_rect.left + (monitor_width - width) / 2;
        self.y = monitor_rect.top + (monitor_height - height) / 2;
        self.width = width;
        self.height = height;
        self.corner_radius = item_size / 4;
        self.icon_size = icon_size;
    }

    pub fn paint(&mut self, state: &SwitchAppsState) {
        let hwnd = self.hwnd;
        let hdc_screen = self.hdc_screen;
        let width = self.width;
        let height = self.height;
        let corner_radius = self.corner_radius as f32;
        let icon_size = self.icon_size;

        let bg_color = self.bg_color;
        let fg_color = self.fg_color;

        unsafe {
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            let bitmap_mem = CreateCompatibleBitmap(hdc_screen, width, height);
            SelectObject(hdc_mem, bitmap_mem);

            let mut graphics = GpGraphics::default();
            let mut graphics_ptr: *mut GpGraphics = &mut graphics;
            GdipCreateFromHDC(hdc_mem, &mut graphics_ptr as _);
            GdipSetSmoothingMode(graphics_ptr, SmoothingModeAntiAlias);

            let mut bg_pen = GpPen::default();
            let mut bg_pen_ptr: *mut GpPen = &mut bg_pen;
            GdipCreatePen1(bg_color, 0.0, Unit(0), &mut bg_pen_ptr as _);

            let mut bg_brush = GpBrush::default();
            let mut bg_brush_ptr: *mut GpBrush = &mut bg_brush;
            GdipGetPenBrushFill(bg_pen_ptr, &mut bg_brush_ptr as _);

            let mut fg_pen = GpPen::default();
            let mut fg_pen_ptr: *mut GpPen = &mut fg_pen;
            GdipCreatePen1(fg_color, 0.0, Unit(0), &mut fg_pen_ptr as _);

            let mut fg_brush = GpBrush::default();
            let mut fg_brush_ptr: *mut GpBrush = &mut fg_brush;
            GdipGetPenBrushFill(fg_pen_ptr, &mut fg_brush_ptr as _);

            draw_round_rect(
                graphics_ptr,
                bg_brush_ptr,
                0.0,
                0.0,
                width as f32,
                height as f32,
                corner_radius,
            );

            let cy = WINDOW_BORDER_SIZE + ICON_BORDER_SIZE;
            let item_size = icon_size + ICON_BORDER_SIZE * 2;
            for (i, (hicon, _)) in state.apps.iter().enumerate() {
                // draw the box for selected icon
                if i == state.index {
                    let left = (item_size * (i as i32) + WINDOW_BORDER_SIZE) as f32;
                    let top = WINDOW_BORDER_SIZE as f32;
                    let right = left + item_size as f32;
                    let bottom = top + item_size as f32;
                    draw_round_rect(
                        graphics_ptr,
                        fg_brush_ptr,
                        left,
                        top,
                        right,
                        bottom,
                        corner_radius,
                    );
                }

                let cx = cy + item_size * (i as i32);

                let mut bitmap = GpBitmap::default();
                let mut bitmap_ptr: *mut GpBitmap = &mut bitmap as _;
                GdipCreateBitmapFromHICON(*hicon, &mut bitmap_ptr as _);

                let image_ptr: *mut GpImage = bitmap_ptr as *mut GpImage;
                GdipDrawImageRect(
                    graphics_ptr,
                    image_ptr,
                    cx as f32,
                    cy as f32,
                    icon_size as f32,
                    icon_size as f32,
                );
            }

            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as _,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as _,
                ..Default::default()
            };
            let _ = UpdateLayeredWindow(
                hwnd,
                hdc_screen,
                Some(&POINT {
                    x: self.x,
                    y: self.y,
                } as _),
                Some(&SIZE {
                    cx: width,
                    cy: height,
                } as _),
                hdc_mem,
                Some(&POINT::default()),
                COLORREF(0),
                Some(&blend as _),
                ULW_ALPHA,
            );

            GdipDeletePen(fg_pen_ptr);
            GdipDeleteBrush(fg_brush_ptr);
            GdipDeletePen(bg_pen_ptr);
            GdipDeleteBrush(bg_brush_ptr);
            GdipDeleteGraphics(graphics_ptr);

            let _ = DeleteObject(bitmap_mem);
            let _ = DeleteDC(hdc_mem);
        }

        if self.show {
            return;
        }
        unsafe {
            if let Ok(hcursor) = LoadCursorW(HMODULE::default(), IDC_ARROW) {
                SetCursor(hcursor);
            }
            let _ = ShowWindow(self.hwnd, SW_SHOW);
            let _ = SetFocus(self.hwnd);
        }
        self.show = true;
    }

    pub fn unpaint(&mut self, state: SwitchAppsState) {
        for (hicon, _) in state.apps {
            let _ = unsafe { DestroyIcon(hicon) };
        }
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
        self.show = false;
    }
}

impl Drop for GdiAAPainter {
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(self.hwnd, self.hdc_screen);
            GdiplusShutdown(self.token);
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

const fn theme_color(light_theme: bool) -> (u32, u32) {
    match light_theme {
        true => (FG_LIGHT_COLOR | ALPHA_MASK, BG_LIGHT_COLOR | ALPHA_MASK),
        false => (FG_DARK_COLOR | ALPHA_MASK, BG_DARK_COLOR | ALPHA_MASK),
    }
}

unsafe fn draw_round_rect(
    graphic_ptr: *mut GpGraphics,
    brush_ptr: *mut GpBrush,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    corner_radius: f32,
) {
    unsafe {
        let mut path = GpPath::default();
        let mut path_ptr: *mut GpPath = &mut path;
        GdipCreatePath(FillModeAlternate, &mut path_ptr as _);
        GdipAddPathArc(
            path_ptr,
            left,
            top,
            corner_radius,
            corner_radius,
            180.0,
            90.0,
        );
        GdipAddPathArc(
            path_ptr,
            right - corner_radius,
            top,
            corner_radius,
            corner_radius,
            270.0,
            90.0,
        );
        GdipAddPathArc(
            path_ptr,
            right - corner_radius,
            bottom - corner_radius,
            corner_radius,
            corner_radius,
            0.0,
            90.0,
        );
        GdipAddPathArc(
            path_ptr,
            left,
            bottom - corner_radius,
            corner_radius,
            corner_radius,
            90.0,
            90.0,
        );
        GdipClosePathFigure(path_ptr);
        GdipFillPath(graphic_ptr, brush_ptr, path_ptr);
        GdipDeletePath(path_ptr);
    }
}
