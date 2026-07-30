use crate::window_grid::{fit_aspect, RectI};
use windows::Win32::{
    Foundation::{COLORREF, HWND},
    Graphics::Gdi::{
        CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetPixel,
        PatBlt, ReleaseDC, SelectObject, SetStretchBltMode, StretchBlt, BLACKNESS, HALFTONE,
        HBITMAP, HDC, HGDIOBJ, SRCCOPY,
    },
    UI::WindowsAndMessaging::{GetWindowPlacement, WINDOWPLACEMENT},
};

const PW_RENDERFULLCONTENT: u32 = 0x00000002;

#[link(name = "user32")]
extern "system" {
    fn PrintWindow(hwnd: HWND, hdc: HDC, flags: u32) -> i32;
}

pub struct OwnedBitmap(HBITMAP);

impl OwnedBitmap {
    pub fn handle(&self) -> HBITMAP {
        self.0
    }
}

impl Drop for OwnedBitmap {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = DeleteObject(self.0.into());
            }
        }
    }
}

struct ScreenDc(HDC);

impl Drop for ScreenDc {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = ReleaseDC(None, self.0);
            }
        }
    }
}

struct SelectedBitmapDc {
    hdc: HDC,
    previous: HGDIOBJ,
}

impl SelectedBitmapDc {
    unsafe fn new(hdc: HDC, bitmap: HBITMAP) -> Option<Self> {
        let previous = SelectObject(hdc, bitmap.into());
        if previous.is_invalid() {
            let _ = DeleteDC(hdc);
            return None;
        }

        Some(Self { hdc, previous })
    }
}

impl Drop for SelectedBitmapDc {
    fn drop(&mut self) {
        unsafe {
            let _ = SelectObject(self.hdc, self.previous);
            let _ = DeleteDC(self.hdc);
        }
    }
}

pub fn capture_window_thumbnail_cached(
    hwnd: HWND,
    target_width: i32,
    target_height: i32,
) -> Option<OwnedBitmap> {
    if target_width <= 0 || target_height <= 0 {
        return None;
    }

    unsafe {
        let (src_w, src_h) = get_window_content_size(hwnd)?;
        if src_w <= 0 || src_h <= 0 {
            return None;
        }

        let hdc_screen = GetDC(None);
        if hdc_screen.is_invalid() {
            return None;
        }
        let _screen_dc = ScreenDc(hdc_screen);

        let bitmap_source = OwnedBitmap(CreateCompatibleBitmap(hdc_screen, src_w, src_h));
        if bitmap_source.handle().is_invalid() {
            return None;
        }
        let hdc_source = CreateCompatibleDC(Some(hdc_screen));
        if hdc_source.is_invalid() {
            return None;
        }
        let _source_dc = SelectedBitmapDc::new(hdc_source, bitmap_source.handle())?;

        if PrintWindow(hwnd, hdc_source, PW_RENDERFULLCONTENT) == 0
            || capture_is_blank(hdc_source, src_w, src_h)
        {
            return None;
        }

        let bitmap_target = OwnedBitmap(CreateCompatibleBitmap(
            hdc_screen,
            target_width,
            target_height,
        ));
        if bitmap_target.handle().is_invalid() {
            return None;
        }
        let hdc_target = CreateCompatibleDC(Some(hdc_screen));
        if hdc_target.is_invalid() {
            return None;
        }
        let _target_dc = SelectedBitmapDc::new(hdc_target, bitmap_target.handle())?;

        if !PatBlt(hdc_target, 0, 0, target_width, target_height, BLACKNESS).as_bool() {
            return None;
        }

        let destination = capture_destination(src_w, src_h, target_width, target_height);
        if SetStretchBltMode(hdc_target, HALFTONE) == 0
            || !StretchBlt(
                hdc_target,
                destination.x,
                destination.y,
                destination.width,
                destination.height,
                Some(hdc_source),
                0,
                0,
                src_w,
                src_h,
                SRCCOPY,
            )
            .as_bool()
        {
            return None;
        }

        Some(bitmap_target)
    }
}

fn capture_destination(source_width: i32, source_height: i32, width: i32, height: i32) -> RectI {
    fit_aspect(source_width, source_height, RectI::new(0, 0, width, height))
}

fn capture_is_blank(hdc: HDC, width: i32, height: i32) -> bool {
    let points = [
        (width / 4, height / 4),
        (width / 2, height / 4),
        (width * 3 / 4, height / 4),
        (width / 4, height / 2),
        (width / 2, height / 2),
        (width * 3 / 4, height / 2),
        (width / 4, height * 3 / 4),
        (width / 2, height * 3 / 4),
        (width * 3 / 4, height * 3 / 4),
    ];
    points
        .into_iter()
        .all(|(x, y)| unsafe { GetPixel(hdc, x, y) } == COLORREF(0))
}

fn window_placement() -> WINDOWPLACEMENT {
    WINDOWPLACEMENT {
        length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
        ..Default::default()
    }
}

unsafe fn get_window_content_size(hwnd: HWND) -> Option<(i32, i32)> {
    let mut placement = window_placement();
    GetWindowPlacement(hwnd, &mut placement).ok()?;
    let rect = placement.rcNormalPosition;
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;
    Some((w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_destination_letterboxes_landscape() {
        assert_eq!(
            capture_destination(2560, 1080, 256, 144),
            RectI::new(0, 18, 256, 108)
        );
    }

    #[test]
    fn capture_destination_pillarboxes_portrait() {
        assert_eq!(
            capture_destination(900, 1600, 256, 144),
            RectI::new(87, 0, 81, 144)
        );
    }

    #[test]
    fn initializes_window_placement_length() {
        assert_eq!(
            window_placement().length as usize,
            std::mem::size_of::<WINDOWPLACEMENT>()
        );
    }
}
