use crate::app::{SwitchAppsState, WindowOverlayItem};
use crate::config::{Config, TitlePosition};
use crate::thumbnail::capture_window_thumbnail;
use crate::utils::{check_error, get_dpi_scale, get_moinitor_rect, is_light_theme, is_win11};

use anyhow::{Context, Result};
use windows::core::PCWSTR;
use windows::Win32::{
    Foundation::{COLORREF, HWND, POINT, RECT, SIZE},
    Graphics::{
        Gdi::{
            CreateCompatibleBitmap, CreateCompatibleDC, CreateRoundRectRgn, CreateSolidBrush,
            DeleteDC, DeleteObject, FillRect, FillRgn, GetDC, ReleaseDC, SelectObject,
            SetStretchBltMode, StretchBlt, AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, HALFTONE,
            HBITMAP, HDC, HPALETTE, SRCCOPY,
        },
        GdiPlus::{
            FillModeAlternate, GdipAddPathArc, GdipClosePathFigure, GdipCreateBitmapFromHBITMAP,
            GdipCreateFont, GdipCreateFontFamilyFromName, GdipCreateFromHDC, GdipCreatePath,
            GdipCreatePen1, GdipCreateStringFormat, GdipDeleteBrush, GdipDeleteFont,
            GdipDeleteFontFamily, GdipDeleteGraphics, GdipDeletePath, GdipDeletePen,
            GdipDeleteStringFormat, GdipDisposeImage, GdipDrawImageRect, GdipDrawLine, GdipDrawString,
            GdipFillPath, GdipFillRectangle, GdipGetPenBrushFill, GdipSetInterpolationMode,
            GdipSetSmoothingMode, GdipSetStringFormatAlign, GdipSetStringFormatLineAlign,
            GdipSetStringFormatTrimming, GdiplusShutdown, GdiplusStartup, GdiplusStartupInput,
            GpBitmap, GpBrush, GpFont, GpFontFamily, GpGraphics, GpImage, GpPath, GpPen,
            GpStringFormat, InterpolationModeHighQualityBicubic, RectF, SmoothingModeAntiAlias,
            StringAlignmentCenter, StringTrimmingEllipsisCharacter, Unit,
        },
    },
    UI::{
        Input::KeyboardAndMouse::SetFocus,
        WindowsAndMessaging::{
            DrawIconEx, GetCursorPos, ShowWindow, UpdateLayeredWindow, DI_NORMAL, SW_HIDE, SW_SHOW,
            ULW_ALPHA,
        },
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbnailHitTarget {
    Card(usize),
    CloseButton(usize),
}

pub const BG_DARK_COLOR: u32 = 0x4c4c4c;
pub const FG_DARK_COLOR: u32 = 0x3b3b3b;
pub const BG_LIGHT_COLOR: u32 = 0xe0e0e0;
pub const FG_LIGHT_COLOR: u32 = 0xf2f2f2;
pub const ALPHA_MASK: u32 = 0xff000000;
pub const ICON_SIZE_BASE: i32 = 64;
pub const WINDOW_BORDER_SIZE_BASE: i32 = 10;
pub const ICON_BORDER_SIZE_BASE: i32 = 4;
pub const SCALE_FACTOR: i32 = 6;
pub const THUMB_WIDTH_BASE: i32 = 256;
pub const THUMB_HEIGHT_BASE: i32 = 144;
pub const THUMB_BORDER_BASE: i32 = 8;

// GDI Antialiasing Painter
pub struct GdiAAPainter {
    token: usize,
    hwnd: HWND,
    hdc_screen: HDC,
    rounded_corner: bool,
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

        let hdc_screen = unsafe { GetDC(Some(hwnd)) };
        let rounded_corner = is_win11();

        Ok(Self {
            token,
            hwnd,
            hdc_screen,
            rounded_corner,
            show: false,
        })
    }

    pub fn paint(&mut self, state: &SwitchAppsState) {
        let dpi_scale = get_dpi_scale(self.hwnd);
        let icon_size_max = (ICON_SIZE_BASE as f64 * dpi_scale) as i32;
        let border_size = (WINDOW_BORDER_SIZE_BASE as f64 * dpi_scale) as i32;
        let icon_border = (ICON_BORDER_SIZE_BASE as f64 * dpi_scale) as i32;

        let Coordinate {
            x,
            y,
            width,
            height,
            icon_size,
            item_size,
        } = Coordinate::new(
            state.apps.len() as i32,
            icon_size_max,
            border_size,
            icon_border,
        );

        let corner_radius = if self.rounded_corner {
            item_size / 4
        } else {
            0
        };

        let hwnd = self.hwnd;
        let hdc_screen = self.hdc_screen;

        let (fg_color, bg_color) = theme_color(is_light_theme());

        unsafe {
            let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
            let bitmap_mem = CreateCompatibleBitmap(hdc_screen, width, height);
            SelectObject(hdc_mem, bitmap_mem.into());

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
                icon_border,
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
                border_size as f32,
                border_size as f32,
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
                Some(hdc_screen),
                Some(&POINT { x, y }),
                Some(&SIZE {
                    cx: width,
                    cy: height,
                }),
                Some(hdc_mem),
                Some(&POINT::default()),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );

            GdipDisposeImage(image_ptr);
            GdipDeleteBrush(bg_brush_ptr);
            GdipDeletePen(bg_pen_ptr);
            GdipDeleteGraphics(graphics_ptr);

            let _ = DeleteDC(hdc_mem);
            let _ = DeleteObject(bitmap_icons.into());
            let _ = DeleteObject(bitmap_mem.into());
        }

        if self.show {
            return;
        }
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOW);
            let _ = SetFocus(Some(self.hwnd));
        }
        self.show = true;
    }

    pub fn unpaint(&mut self, _state: SwitchAppsState) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
        self.show = false;
    }

    pub fn find_clicked_app_index(&self, state: &SwitchAppsState) -> Option<usize> {
        let cursor_pos = unsafe {
            let mut pos = POINT::default();
            let _ = GetCursorPos(&mut pos);
            pos
        };

        let dpi_scale = get_dpi_scale(self.hwnd);
        let icon_size_max = (ICON_SIZE_BASE as f64 * dpi_scale) as i32;
        let border_size = (WINDOW_BORDER_SIZE_BASE as f64 * dpi_scale) as i32;
        let icon_border = (ICON_BORDER_SIZE_BASE as f64 * dpi_scale) as i32;

        let Coordinate {
            x, y, item_size, ..
        } = Coordinate::new(
            state.apps.len() as i32,
            icon_size_max,
            border_size,
            icon_border,
        );

        let xpos = cursor_pos.x - x;
        let ypos = cursor_pos.y - y;

        let cy = border_size;
        for (i, _) in state.apps.iter().enumerate() {
            let cx = border_size + item_size * (i as i32);
            if xpos >= cx && xpos < cx + item_size && ypos >= cy && ypos < cy + item_size {
                return Some(i);
            }
        }
        None
    }

    pub fn paint_switch_windows(
        &mut self,
        windows: &[WindowOverlayItem],
        index: usize,
        config: &Config,
        hovered_close_index: Option<usize>,
    ) {
        if windows.is_empty() {
            return;
        }

        let dpi_scale = get_dpi_scale(self.hwnd);
        let border_size = (WINDOW_BORDER_SIZE_BASE as f64 * dpi_scale) as i32;
        let thumb_border = (THUMB_BORDER_BASE as f64 * dpi_scale) as i32;

        let coord = ThumbCoordinate::new(windows.len() as i32, border_size, thumb_border, dpi_scale, config);
        let corner_radius = if self.rounded_corner {
            coord.thumb_height / 8
        } else {
            0
        };

        let thumb_top_offset = if config.switch_windows_show_title
            && config.switch_windows_title_position == TitlePosition::Top
        {
            coord.title_height
        } else {
            0
        };

        let hwnd = self.hwnd;
        let hdc_screen = self.hdc_screen;
        let (fg_color, bg_color) = theme_color(is_light_theme());

        unsafe {
            let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
            let bitmap_mem = CreateCompatibleBitmap(hdc_screen, coord.width, coord.height);
            SelectObject(hdc_mem, bitmap_mem.into());

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
                    coord.width as f32,
                    coord.height as f32,
                    corner_radius as f32,
                );
            } else {
                GdipFillRectangle(
                    graphics_ptr,
                    bg_brush_ptr,
                    0.0,
                    0.0,
                    coord.width as f32,
                    coord.height as f32,
                );
            }

            let fg_brush = CreateSolidBrush(COLORREF(fg_color));

            let mut font_family = GpFontFamily::default();
            let mut font_family_ptr: *mut GpFontFamily = &mut font_family;
            let mut font = GpFont::default();
            let mut font_ptr: *mut GpFont = &mut font;
            let mut string_format = GpStringFormat::default();
            let mut string_format_ptr: *mut GpStringFormat = &mut string_format;

            if config.switch_windows_show_title {
                let font_name = PCWSTR(windows::core::w!("Segoe UI").as_ptr());
                GdipCreateFontFamilyFromName(font_name, std::ptr::null_mut(), &mut font_family_ptr as _);
                let font_size = (11.0 * dpi_scale) as f32;
                GdipCreateFont(
                    font_family_ptr,
                    font_size,
                    0,
                    Unit(2),
                    &mut font_ptr as _,
                );
                GdipCreateStringFormat(0, 0, &mut string_format_ptr as _);
                GdipSetStringFormatAlign(string_format_ptr, StringAlignmentCenter);
                GdipSetStringFormatLineAlign(string_format_ptr, StringAlignmentCenter);
                GdipSetStringFormatTrimming(string_format_ptr, StringTrimmingEllipsisCharacter);
            }

            for (i, item) in windows.iter().enumerate() {
                let cx = coord.border_size + coord.thumb_item_width * (i as i32);
                let cy = coord.border_size;

                let thumbnail = capture_window_thumbnail(
                    item.hwnd,
                    hdc_screen,
                    coord.thumb_width,
                    coord.thumb_height,
                );

                let mut bitmap = GpBitmap::default();
                let mut bitmap_ptr: *mut GpBitmap = &mut bitmap;
                GdipCreateBitmapFromHBITMAP(thumbnail, HPALETTE::default(), &mut bitmap_ptr as _);
                let image_ptr: *mut GpImage = bitmap_ptr as *mut GpImage;

                if i == index {
                    let rgn = CreateRoundRectRgn(
                        cx,
                        cy,
                        cx + coord.thumb_item_width,
                        cy + coord.thumb_item_height,
                        corner_radius,
                        corner_radius,
                    );
                    let _ = FillRgn(hdc_mem, rgn, fg_brush);
                    let _ = DeleteObject(rgn.into());
                }

                let thumb_x = cx + thumb_border;
                let thumb_y = cy + thumb_border + thumb_top_offset;

                GdipDrawImageRect(
                    graphics_ptr,
                    image_ptr,
                    thumb_x as f32,
                    thumb_y as f32,
                    coord.thumb_width as f32,
                    coord.thumb_height as f32,
                );

                GdipDisposeImage(image_ptr);
                let _ = DeleteObject(thumbnail.into());

                if config.switch_windows_show_title {
                    let (rect, title_color) = match config.switch_windows_title_position {
                        TitlePosition::Bottom => (
                            RectF {
                                X: thumb_x as f32,
                                Y: (thumb_y + coord.thumb_height + 2) as f32,
                                Width: coord.thumb_width as f32,
                                Height: (coord.title_height - 2) as f32,
                            },
                            ALPHA_MASK | fg_color,
                        ),
                        TitlePosition::Top => (
                            RectF {
                                X: thumb_x as f32,
                                Y: (cy + thumb_border) as f32,
                                Width: coord.thumb_width as f32,
                                Height: (coord.title_height - 2) as f32,
                            },
                            ALPHA_MASK | fg_color,
                        ),
                        TitlePosition::Overlay => {
                            let overlay_h = (24.0 * dpi_scale) as f32;
                            let overlay_y = (thumb_y + coord.thumb_height) as f32 - overlay_h;

                            let mut dark_pen = GpPen::default();
                            let mut dark_pen_ptr: *mut GpPen = &mut dark_pen;
                            GdipCreatePen1(0xaa000000, 0.0, Unit(0), &mut dark_pen_ptr as _);
                            let mut dark_brush = GpBrush::default();
                            let mut dark_brush_ptr: *mut GpBrush = &mut dark_brush;
                            GdipGetPenBrushFill(dark_pen_ptr, &mut dark_brush_ptr as _);

                            draw_round_rect(
                                graphics_ptr,
                                dark_brush_ptr,
                                thumb_x as f32,
                                overlay_y,
                                (thumb_x + coord.thumb_width) as f32,
                                overlay_y + overlay_h,
                                (4.0 * dpi_scale) as f32,
                            );

                            GdipDeleteBrush(dark_brush_ptr);
                            GdipDeletePen(dark_pen_ptr);

                            (
                                RectF {
                                    X: thumb_x as f32,
                                    Y: overlay_y,
                                    Width: coord.thumb_width as f32,
                                    Height: overlay_h,
                                },
                                0xffffffff,
                            )
                        }
                    };

                    let mut text_pen = GpPen::default();
                    let mut text_pen_ptr: *mut GpPen = &mut text_pen;
                    GdipCreatePen1(title_color, 0.0, Unit(0), &mut text_pen_ptr as _);
                    let mut text_brush = GpBrush::default();
                    let mut text_brush_ptr: *mut GpBrush = &mut text_brush;
                    GdipGetPenBrushFill(text_pen_ptr, &mut text_brush_ptr as _);

                    let title_u16: Vec<u16> = item.title.encode_utf16().collect();
                    GdipDrawString(
                        graphics_ptr,
                        PCWSTR(title_u16.as_ptr()),
                        title_u16.len() as i32,
                        font_ptr,
                        &rect,
                        string_format_ptr,
                        text_brush_ptr,
                    );

                    GdipDeleteBrush(text_brush_ptr);
                    GdipDeletePen(text_pen_ptr);
                }

                if config.switch_windows_show_app_icon {
                    if let Some(hicon) = item.hicon {
                        let icon_badge_size = (20.0 * dpi_scale) as i32;
                        let _ = DrawIconEx(
                            hdc_mem,
                            cx + thumb_border + 4,
                            cy + thumb_border + thumb_top_offset + 4,
                            hicon,
                            icon_badge_size,
                            icon_badge_size,
                            0,
                            None,
                            DI_NORMAL,
                        );
                    }
                }

                if config.switch_windows_enable_quick_actions {
                    let close_size = (18.0 * dpi_scale) as i32;
                    let close_x = cx + coord.thumb_item_width - close_size - thumb_border;
                    let close_y = cy + thumb_border + thumb_top_offset;

                    let close_bg_color = if hovered_close_index == Some(i) {
                        0xffe81123
                    } else {
                        0x88000000
                    };

                    let mut close_pen = GpPen::default();
                    let mut close_pen_ptr: *mut GpPen = &mut close_pen;
                    GdipCreatePen1(close_bg_color, 0.0, Unit(0), &mut close_pen_ptr as _);
                    let mut close_brush = GpBrush::default();
                    let mut close_brush_ptr: *mut GpBrush = &mut close_brush;
                    GdipGetPenBrushFill(close_pen_ptr, &mut close_brush_ptr as _);

                    draw_round_rect(
                        graphics_ptr,
                        close_brush_ptr,
                        close_x as f32,
                        close_y as f32,
                        (close_x + close_size) as f32,
                        (close_y + close_size) as f32,
                        close_size as f32,
                    );

                    GdipDeleteBrush(close_brush_ptr);
                    GdipDeletePen(close_pen_ptr);

                    let mut x_pen = GpPen::default();
                    let mut x_pen_ptr: *mut GpPen = &mut x_pen;
                    GdipCreatePen1(0xffffffff, (1.5 * dpi_scale) as f32, Unit(0), &mut x_pen_ptr as _);
                    let p = (5.0 * dpi_scale) as f32;
                    GdipDrawLine(
                        graphics_ptr,
                        x_pen_ptr,
                        close_x as f32 + p,
                        close_y as f32 + p,
                        (close_x + close_size) as f32 - p,
                        (close_y + close_size) as f32 - p,
                    );
                    GdipDrawLine(
                        graphics_ptr,
                        x_pen_ptr,
                        (close_x + close_size) as f32 - p,
                        close_y as f32 + p,
                        close_x as f32 + p,
                        (close_y + close_size) as f32 - p,
                    );
                    GdipDeletePen(x_pen_ptr);
                }
            }

            if config.switch_windows_show_title {
                GdipDeleteStringFormat(string_format_ptr);
                GdipDeleteFont(font_ptr);
                GdipDeleteFontFamily(font_family_ptr);
            }

            let _ = DeleteObject(fg_brush.into());
            GdipDeleteBrush(bg_brush_ptr);
            GdipDeletePen(bg_pen_ptr);
            GdipDeleteGraphics(graphics_ptr);

            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as _,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as _,
                ..Default::default()
            };
            let _ = UpdateLayeredWindow(
                hwnd,
                Some(hdc_screen),
                Some(&POINT { x: coord.x, y: coord.y }),
                Some(&SIZE {
                    cx: coord.width,
                    cy: coord.height,
                }),
                Some(hdc_mem),
                Some(&POINT::default()),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );

            let _ = DeleteDC(hdc_mem);
            let _ = DeleteObject(bitmap_mem.into());
        }

        if self.show {
            return;
        }
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOW);
            let _ = SetFocus(Some(self.hwnd));
        }
        self.show = true;
    }

    pub fn unpaint_switch_windows(&mut self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
        self.show = false;
    }

    pub fn hit_test_switch_windows(&self, windows_len: usize, config: &Config) -> Option<ThumbnailHitTarget> {
        if windows_len == 0 {
            return None;
        }
        let cursor_pos = unsafe {
            let mut pos = POINT::default();
            let _ = GetCursorPos(&mut pos);
            pos
        };

        let dpi_scale = get_dpi_scale(self.hwnd);
        let border_size = (WINDOW_BORDER_SIZE_BASE as f64 * dpi_scale) as i32;
        let thumb_border = (THUMB_BORDER_BASE as f64 * dpi_scale) as i32;
        let coord = ThumbCoordinate::new(windows_len as i32, border_size, thumb_border, dpi_scale, config);

        let xpos = cursor_pos.x - coord.x;
        let ypos = cursor_pos.y - coord.y;
        let close_size = (18.0 * dpi_scale) as i32;

        let thumb_top_offset = if config.switch_windows_show_title
            && config.switch_windows_title_position == TitlePosition::Top
        {
            coord.title_height
        } else {
            0
        };

        for i in 0..windows_len {
            let cx = coord.border_size + coord.thumb_item_width * (i as i32);
            let cy = coord.border_size;

            if xpos >= cx
                && xpos < cx + coord.thumb_item_width
                && ypos >= cy
                && ypos < cy + coord.thumb_item_height
            {
                if config.switch_windows_enable_quick_actions {
                    let close_x = cx + coord.thumb_item_width - close_size - thumb_border;
                    let close_y = cy + thumb_border + thumb_top_offset;
                    if xpos >= close_x && xpos < close_x + close_size && ypos >= close_y && ypos < close_y + close_size {
                        return Some(ThumbnailHitTarget::CloseButton(i));
                    }
                }
                return Some(ThumbnailHitTarget::Card(i));
            }
        }
        None
    }
}

impl Drop for GdiAAPainter {
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(Some(self.hwnd), self.hdc_screen);
            GdiplusShutdown(self.token);
        }
    }
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
        if corner_radius <= 0.0 {
            GdipFillRectangle(
                graphic_ptr,
                brush_ptr,
                left,
                top,
                right - left,
                bottom - top,
            );
            return;
        }

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
    icon_border: i32,
    width: i32,
    height: i32,
    corner_radius: i32,
    fg_color: u32,
    bg_color: u32,
) -> HBITMAP {
    let scaled_width = width * SCALE_FACTOR;
    let scaled_height = height * SCALE_FACTOR;
    let scaled_corner_radius = corner_radius * SCALE_FACTOR;
    let scaled_border_size = icon_border * SCALE_FACTOR;
    let scaled_icon_inner_size = icon_size * SCALE_FACTOR;
    let scaled_icon_outer_size = scaled_icon_inner_size + scaled_border_size * 2;

    unsafe {
        let hdc_tmp = CreateCompatibleDC(Some(hdc_screen));
        let bitmap_tmp = CreateCompatibleBitmap(hdc_screen, width, height);
        SelectObject(hdc_tmp, bitmap_tmp.into());

        let hdc_scaled = CreateCompatibleDC(Some(hdc_screen));
        let bitmap_scaled = CreateCompatibleBitmap(hdc_screen, scaled_width, scaled_height);
        SelectObject(hdc_scaled, bitmap_scaled.into());

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
                let _ = DeleteObject(rgn.into());
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
                None,
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
            Some(hdc_scaled),
            0,
            0,
            scaled_width,
            scaled_height,
            SRCCOPY,
        );

        let _ = DeleteObject(fg_brush.into());
        let _ = DeleteObject(bg_brush.into());
        let _ = DeleteDC(hdc_scaled);
        let _ = DeleteDC(hdc_tmp);
        let _ = DeleteObject(bitmap_scaled.into());

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
    fn new(num_apps: i32, icon_size_max: i32, border_size: i32, icon_border: i32) -> Self {
        let monitor_rect = get_moinitor_rect();
        let monitor_width = monitor_rect.right - monitor_rect.left;
        let monitor_height = monitor_rect.bottom - monitor_rect.top;
        let n_apps = num_apps.max(1);

        let icon_size =
            ((monitor_width - 2 * border_size) / n_apps - icon_border * 2).min(icon_size_max).max(1);

        let item_size = icon_size + icon_border * 2;
        let width = item_size * num_apps + border_size * 2;
        let height = item_size + border_size * 2;
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

struct ThumbCoordinate {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    thumb_width: i32,
    thumb_height: i32,
    thumb_item_width: i32,
    thumb_item_height: i32,
    border_size: i32,
    title_height: i32,
}

impl ThumbCoordinate {
    fn new(num_items: i32, border_size: i32, thumb_border: i32, dpi_scale: f64, config: &Config) -> Self {
        let monitor_rect = get_moinitor_rect();
        let monitor_width = monitor_rect.right - monitor_rect.left;
        let monitor_height = monitor_rect.bottom - monitor_rect.top;
        let n_items = num_items.max(1);

        let thumb_width_max = (THUMB_WIDTH_BASE as f64 * dpi_scale) as i32;
        let thumb_height_max = (THUMB_HEIGHT_BASE as f64 * dpi_scale) as i32;

        let raw_width = (monitor_width - 2 * border_size) / n_items - thumb_border * 2;
        let thumb_width = raw_width.min(thumb_width_max).max(1);
        let thumb_height = thumb_width * thumb_height_max / thumb_width_max;

        let title_height = if config.switch_windows_show_title
            && config.switch_windows_title_position != TitlePosition::Overlay
        {
            (24.0 * dpi_scale) as i32
        } else {
            0
        };

        let thumb_item_width = thumb_width + thumb_border * 2;
        let thumb_item_height = thumb_height + thumb_border * 2 + title_height;

        let width = thumb_item_width * num_items + border_size * 2;
        let height = thumb_item_height + border_size * 2;
        let x = monitor_rect.left + (monitor_width - width) / 2;
        let y = monitor_rect.top + (monitor_height - height) / 2;

        Self {
            x,
            y,
            width,
            height,
            thumb_width,
            thumb_height,
            thumb_item_width,
            thumb_item_height,
            border_size,
            title_height,
        }
    }
}
