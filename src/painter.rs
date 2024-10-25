use crate::app::SwitchAppsState;
use crate::utils::{check_error, get_moinitor_rect, is_light_theme, is_win11};

use anyhow::{Context, Result};
use windows::Win32::Foundation::{COLORREF, POINT, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleBitmap, CreateCompatibleDC, CreateRoundRectRgn, CreateSolidBrush, DeleteDC,
    DeleteObject, FillRect, FillRgn, ReleaseDC, SelectObject, SetStretchBltMode, StretchBlt,
    AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, HALFTONE, HBITMAP, HBRUSH, HDC, HPALETTE, SRCCOPY,
};
use windows::Win32::Graphics::GdiPlus::{
    FillModeAlternate, GdipAddPathArc, GdipClosePathFigure, GdipCreateBitmapFromHBITMAP,
    GdipCreateFromHDC, GdipCreatePath, GdipCreatePen1, GdipDeleteBrush, GdipDeleteGraphics,
    GdipDeletePath, GdipDeletePen, GdipDisposeImage, GdipDrawImageRect, GdipFillPath,
    GdipFillRectangle, GdipGetPenBrushFill, GdipSetInterpolationMode, GdipSetSmoothingMode,
    GdiplusShutdown, GdiplusStartup, GdiplusStartupInput, GpBitmap, GpBrush, GpGraphics, GpImage,
    GpPath, GpPen, InterpolationModeHighQualityBicubic, SmoothingModeAntiAlias, Unit,
};
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    DrawIconEx, GetCursorPos, ShowWindow, UpdateLayeredWindow, DI_NORMAL, SW_HIDE, SW_SHOW,
    ULW_ALPHA,
};
use windows::Win32::{Foundation::HWND, Graphics::Gdi::GetDC};

pub const BG_DARK_COLOR: u32 = 0x4c4c4c;
pub const FG_DARK_COLOR: u32 = 0x3b3b3b;
pub const BG_LIGHT_COLOR: u32 = 0xe0e0e0;
pub const FG_LIGHT_COLOR: u32 = 0xf2f2f2;
pub const ALPHA_MASK: u32 = 0xff000000;
pub const ICON_SIZE: i32 = 64;
pub const WINDOW_BORDER_SIZE: i32 = 10;
pub const ICON_BORDER_SIZE: i32 = 4;
pub const SCALE_FACTOR: i32 = 6;

// GDI Antialiasing Painter
pub struct GdiAAPainter {
    token: usize,
    hwnd: HWND,
    hdc_screen: HDC,
    rounded_corner: bool,
    light: bool,
    show: bool,
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
        let rounded_corner = is_win11();
        let light = is_light_theme();

        Ok(Self {
            token,
            hwnd,
            hdc_screen,
            rounded_corner,
            light,
            show: false,
        })
    }

    pub fn paint(&mut self, state: &SwitchAppsState) {
        let Coordinate {
            x,
            y,
            width,
            height,
            icon_size,
            item_size,
        } = Coordinate::new(state.apps.len() as i32);

        let corner_radius = if self.rounded_corner {
            item_size / 4
        } else {
            0
        };

        let hwnd = self.hwnd;
        let hdc_screen = self.hdc_screen;

        let (fg_color, bg_color) = theme_color(self.light);

        unsafe {
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            let bitmap_mem = CreateCompatibleBitmap(hdc_screen, width, height);
            SelectObject(hdc_mem, bitmap_mem);

            let mut graphics = GpGraphics::default();
            let mut graphics_ptr: *mut GpGraphics = &mut graphics;
            GdipCreateFromHDC(hdc_mem, &mut graphics_ptr as _);
            GdipSetSmoothingMode(graphics_ptr, SmoothingModeAntiAlias);
            GdipSetInterpolationMode(graphics_ptr, InterpolationModeHighQualityBicubic);

            let mut bg_pen = GpPen::default();
            let mut bg_pen_ptr: *mut GpPen = &mut bg_pen;
            GdipCreatePen1(ALPHA_MASK | bg_color, 0.0, Unit(0), &mut bg_pen_ptr as _);

            let mut bg_brush = GpBrush::default();
            let mut bg_brush_ptr: *mut GpBrush = &mut bg_brush;
            GdipGetPenBrushFill(bg_pen_ptr, &mut bg_brush_ptr as _);

            if self.rounded_corner {
                draw_round_rect(
                    graphics_ptr,
                    bg_brush_ptr,
                    0.0,
                    0.0,
                    width as f32,
                    height as f32,
                    corner_radius as f32,
                );
            } else {
                GdipFillRectangle(
                    graphics_ptr,
                    bg_brush_ptr,
                    0.0,
                    0.0,
                    width as f32,
                    height as f32,
                );
            }

            let icons_width = item_size * state.apps.len() as i32;
            let icons_height = item_size;
            let bitmap_icons = draw_icons(
                state,
                hdc_screen,
                icon_size,
                icons_width,
                icons_height,
                corner_radius,
                fg_color,
                bg_color,
            );

            let mut bitmap = GpBitmap::default();
            let mut bitmap_ptr: *mut GpBitmap = &mut bitmap as _;
            GdipCreateBitmapFromHBITMAP(bitmap_icons, HPALETTE::default(), &mut bitmap_ptr as _);

            let image_ptr: *mut GpImage = bitmap_ptr as *mut GpImage;
            GdipDrawImageRect(
                graphics_ptr,
                image_ptr,
                WINDOW_BORDER_SIZE as f32,
                WINDOW_BORDER_SIZE as f32,
                icons_width as f32,
                icons_height as f32,
            );

            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as _,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as _,
                ..Default::default()
            };
            let _ = UpdateLayeredWindow(
                hwnd,
                hdc_screen,
                Some(&POINT { x, y }),
                Some(&SIZE {
                    cx: width,
                    cy: height,
                }),
                hdc_mem,
                Some(&POINT::default()),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );

            GdipDisposeImage(image_ptr);
            GdipDeleteBrush(bg_brush_ptr);
            GdipDeletePen(bg_pen_ptr);
            GdipDeleteGraphics(graphics_ptr);

            let _ = DeleteObject(bitmap_icons);
            let _ = DeleteObject(bitmap_mem);
            let _ = DeleteDC(hdc_mem);
        }

        if self.show {
            return;
        }
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOW);
            let _ = SetFocus(self.hwnd);
        }
        self.show = true;
    }

    pub fn unpaint(&mut self, _state: SwitchAppsState) {
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

pub fn find_clicked_app_index(state: &SwitchAppsState) -> Option<usize> {
    let Coordinate {
        x, y, item_size, ..
    } = Coordinate::new(state.apps.len() as i32);

    let mut cursor_pos = POINT::default();
    let _ = unsafe { GetCursorPos(&mut cursor_pos) };

    let xpos = cursor_pos.x - x;
    let ypos = cursor_pos.y - y;

    let cy = WINDOW_BORDER_SIZE;
    for (i, _) in state.apps.iter().enumerate() {
        let cx = WINDOW_BORDER_SIZE + item_size * (i as i32);
        if xpos >= cx && xpos < cx + item_size && ypos >= cy && ypos < cy + item_size {
            return Some(i);
        }
    }
    None
}

const fn theme_color(light_theme: bool) -> (u32, u32) {
    match light_theme {
        true => (FG_LIGHT_COLOR, BG_LIGHT_COLOR),
        false => (FG_DARK_COLOR, BG_DARK_COLOR),
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

#[allow(clippy::too_many_arguments)]
fn draw_icons(
    state: &SwitchAppsState,
    hdc_screen: HDC,
    icon_size: i32,
    width: i32,
    height: i32,
    corner_radius: i32,
    fg_color: u32,
    bg_color: u32,
) -> HBITMAP {
    let scaled_width = width * SCALE_FACTOR;
    let scaled_height = height * SCALE_FACTOR;
    let scaled_corner_radius = corner_radius * SCALE_FACTOR;
    let scaled_border_size = ICON_BORDER_SIZE * SCALE_FACTOR;
    let scaled_icon_inner_size = icon_size * SCALE_FACTOR;
    let scaled_icon_outer_size = scaled_icon_inner_size + scaled_border_size * 2;

    unsafe {
        let hdc_tmp = CreateCompatibleDC(hdc_screen);
        let bitmap_tmp = CreateCompatibleBitmap(hdc_screen, width, height);
        SelectObject(hdc_tmp, bitmap_tmp);

        let hdc_scaled = CreateCompatibleDC(hdc_screen);
        let bitmap_scaled = CreateCompatibleBitmap(hdc_screen, scaled_width, scaled_height);
        SelectObject(hdc_scaled, bitmap_scaled);

        let fg_brush = CreateSolidBrush(COLORREF(fg_color));
        let bg_brush = CreateSolidBrush(COLORREF(bg_color));

        let rect = RECT {
            left: 0,
            top: 0,
            right: scaled_width,
            bottom: scaled_height,
        };

        FillRect(hdc_scaled, &rect, bg_brush);

        for (i, (icon, _)) in state.apps.iter().enumerate() {
            // draw the box for selected icon
            if i == state.index {
                let left = scaled_icon_outer_size * (i as i32);
                let top = 0;
                let right = left + scaled_icon_outer_size;
                let bottom = top + scaled_icon_outer_size;
                let rgn = CreateRoundRectRgn(
                    left,
                    top,
                    right,
                    bottom,
                    scaled_corner_radius,
                    scaled_corner_radius,
                );
                let _ = FillRgn(hdc_scaled, rgn, fg_brush);
                let _ = DeleteObject(rgn);
            }

            let cx = scaled_border_size + scaled_icon_outer_size * (i as i32);
            let _ = DrawIconEx(
                hdc_scaled,
                cx,
                scaled_border_size,
                *icon,
                scaled_icon_inner_size,
                scaled_icon_inner_size,
                0,
                HBRUSH::default(),
                DI_NORMAL,
            );
        }

        SetStretchBltMode(hdc_tmp, HALFTONE);
        let _ = StretchBlt(
            hdc_tmp,
            0,
            0,
            width,
            height,
            hdc_scaled,
            0,
            0,
            scaled_width,
            scaled_height,
            SRCCOPY,
        );

        let _ = DeleteObject(fg_brush);
        let _ = DeleteObject(bg_brush);
        let _ = DeleteObject(bitmap_scaled);
        let _ = DeleteDC(hdc_scaled);
        let _ = DeleteDC(hdc_tmp);

        bitmap_tmp
    }
}

struct Coordinate {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    icon_size: i32,
    item_size: i32,
}

impl Coordinate {
    fn new(num_apps: i32) -> Self {
        let monitor_rect = get_moinitor_rect();
        let monitor_width = monitor_rect.right - monitor_rect.left;
        let monitor_height = monitor_rect.bottom - monitor_rect.top;

        let icon_size = ((monitor_width - 2 * WINDOW_BORDER_SIZE) / num_apps
            - ICON_BORDER_SIZE * 2)
            .min(ICON_SIZE);

        let item_size = icon_size + ICON_BORDER_SIZE * 2;
        let width = item_size * num_apps + WINDOW_BORDER_SIZE * 2;
        let height = item_size + WINDOW_BORDER_SIZE * 2;
        let x = monitor_rect.left + (monitor_width - width) / 2;
        let y = monitor_rect.top + (monitor_height - height) / 2;

        Self {
            x,
            y,
            width,
            height,
            icon_size,
            item_size,
        }
    }
}
