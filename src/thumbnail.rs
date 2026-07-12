use windows::Win32::{
    Foundation::HWND,
    Graphics::Gdi::{
        CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, SelectObject,
        SetStretchBltMode, StretchBlt, HALFTONE, HBITMAP, HDC, SRCCOPY,
    },
    UI::WindowsAndMessaging::{
        DrawIconEx, GetClassLongPtrW, GetWindowPlacement, DI_NORMAL, GCLP_HICON, WINDOWPLACEMENT,
    },
};

const PW_RENDERFULLCONTENT: u32 = 0x00000002;

#[link(name = "user32")]
extern "system" {
    fn PrintWindow(hwnd: HWND, hdc: HDC, flags: u32) -> i32;
}

pub fn capture_window_thumbnail(
    hwnd: HWND,
    hdc_screen: HDC,
    target_width: i32,
    target_height: i32,
) -> HBITMAP {
    unsafe {
        let (src_w, src_h) = get_window_content_size(hwnd);
        if src_w <= 0 || src_h <= 0 {
            return capture_icon_fallback(hwnd, hdc_screen, target_width, target_height);
        }

        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        let bitmap_mem = CreateCompatibleBitmap(hdc_screen, src_w, src_h);
        SelectObject(hdc_mem, bitmap_mem.into());

        let success = PrintWindow(hwnd, hdc_mem, PW_RENDERFULLCONTENT);
        if success != 0 {
            let hdc_target = CreateCompatibleDC(Some(hdc_screen));
            let bitmap_target = CreateCompatibleBitmap(hdc_screen, target_width, target_height);
            SelectObject(hdc_target, bitmap_target.into());

            SetStretchBltMode(hdc_target, HALFTONE);
            let _ = StretchBlt(
                hdc_target,
                0,
                0,
                target_width,
                target_height,
                Some(hdc_mem),
                0,
                0,
                src_w,
                src_h,
                SRCCOPY,
            );

            let _ = DeleteDC(hdc_mem);
            let _ = DeleteObject(bitmap_mem.into());
            let _ = DeleteDC(hdc_target);

            return bitmap_target;
        }

        let _ = DeleteDC(hdc_mem);
        let _ = DeleteObject(bitmap_mem.into());

        capture_icon_fallback(hwnd, hdc_screen, target_width, target_height)
    }
}

unsafe fn get_window_content_size(hwnd: HWND) -> (i32, i32) {
    let mut placement = WINDOWPLACEMENT::default();
    let _ = GetWindowPlacement(hwnd, &mut placement);
    let rect = placement.rcNormalPosition;
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;
    (w, h)
}

unsafe fn capture_icon_fallback(
    hwnd: HWND,
    hdc_screen: HDC,
    target_width: i32,
    target_height: i32,
) -> HBITMAP {
    let hdc_target = CreateCompatibleDC(Some(hdc_screen));
    let bitmap_target = CreateCompatibleBitmap(hdc_screen, target_width, target_height);
    SelectObject(hdc_target, bitmap_target.into());

    let hicon = GetClassLongPtrW(hwnd, GCLP_HICON);
    if hicon != 0 {
        let icon_size = target_width.min(target_height) * 3 / 4;
        let x = (target_width - icon_size) / 2;
        let y = (target_height - icon_size) / 2;
        let _ = DrawIconEx(
            hdc_target,
            x,
            y,
            windows::Win32::UI::WindowsAndMessaging::HICON(hicon as _),
            icon_size,
            icon_size,
            0,
            None,
            DI_NORMAL,
        );
    }

    let _ = DeleteDC(hdc_target);
    bitmap_target
}
