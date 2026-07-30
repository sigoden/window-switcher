use crate::{thumbnail::OwnedBitmap, window_grid::WindowGridLayout};
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{IsWindow, HICON},
};

pub(crate) struct WindowOverlayItem {
    #[allow(dead_code)]
    pub hwnd: HWND,
    pub title: String,
    pub thumbnail: Option<OwnedBitmap>,
}

pub(crate) struct WindowOverlaySession {
    pub app_name: String,
    pub app_icon: HICON,
    #[allow(dead_code)]
    pub original_hwnd: HWND,
    pub items: Vec<WindowOverlayItem>,
    pub selected: usize,
    pub hovered: Option<usize>,
    pub hovered_close: Option<usize>,
    pub layout: WindowGridLayout,
}

impl WindowOverlaySession {
    #[allow(dead_code)]
    pub fn prune_invalid(&mut self) {
        self.items
            .retain(|item| unsafe { IsWindow(Some(item.hwnd)).as_bool() });
        if !self.items.is_empty() {
            self.selected = self.selected.min(self.items.len() - 1);
        }
    }
}
