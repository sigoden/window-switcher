# DockDoor Window Titles and Previews Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add macOS DockDoor-style window titles, app icon badges, and quick-action window closing to the `window-switcher` preview overlay, fully configurable via `window-switcher.ini`.

**Architecture:** Extend INI configuration loading in `src/config.rs` with `show_title`, `title_position`, `show_app_icon`, and `enable_quick_actions`. Update `src/app.rs` to track window overlay items (`WindowOverlayItem`) with app icons and handle window close actions (`WM_CLOSE`). Update `src/painter.rs` to dynamically calculate layout geometry and render text titles (GDI+ `GdipDrawString`), app icon badges (`DrawIconEx`), and interactive close buttons.

**Tech Stack:** Rust, Win32 API (`windows` crate), GDI+ (`GpGraphics`, `GdipDrawString`), INI parsing (`ini` crate).

---

### File Structure Map

- `window-switcher.ini`: Default configuration file containing new `[switch-windows]` options.
- `src/config.rs`: INI parser, `TitlePosition` enum, `Config` struct fields, and unit tests.
- `src/app.rs`: State management (`WindowOverlayItem`, `SwitchWindowsState`), message loop integration (`WM_LBUTTONUP`, `WM_MBUTTONUP`, `WM_MOUSEMOVE`), and `WM_CLOSE` actions.
- `src/painter.rs`: Layout calculation (`ThumbCoordinate`), text rendering (`GdipDrawString`), app icon badge drawing (`DrawIconEx`), and hit-testing (`ThumbnailHitTarget`).

---

### Task 1: Update Configuration (`window-switcher.ini` and `src/config.rs`)

**Files:**
- Modify: `window-switcher.ini:16-18`
- Modify: `src/config.rs:16-105`
- Test: `src/config.rs` (unit tests block at bottom)

- [ ] **Step 1: Write failing unit tests for new config options**

Add tests to `src/config.rs`:

```rust
#[test]
fn test_config_title_position_parse() {
    let ini_str = r#"
[switch-windows]
show_title = yes
title_position = overlay
show_app_icon = no
enable_quick_actions = yes
"#;
    let ini = Ini::load_from_str(ini_str).unwrap();
    let config = Config::load(&ini).unwrap();
    assert!(config.switch_windows_show_title);
    assert_eq!(config.switch_windows_title_position, TitlePosition::Overlay);
    assert!(!config.switch_windows_show_app_icon);
    assert!(config.switch_windows_enable_quick_actions);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --target=x86_64-pc-windows-msvc test_config_title_position_parse`
Expected: FAIL due to missing fields and `TitlePosition` enum.

- [ ] **Step 3: Implement `TitlePosition` enum and `Config` fields**

In `src/config.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TitlePosition {
    #[default]
    Bottom,
    Top,
    Overlay,
}
```

Add fields to `Config` struct and default values:
- `pub switch_windows_show_title: bool` (default: `true`)
- `pub switch_windows_title_position: TitlePosition` (default: `TitlePosition::Bottom`)
- `pub switch_windows_show_app_icon: bool` (default: `true`)
- `pub switch_windows_enable_quick_actions: bool` (default: `true`)

Parse options in `Config::load`:
```rust
if let Some(v) = section.get("show_title").and_then(Config::to_bool) {
    conf.switch_windows_show_title = v;
}
if let Some(v) = section.get("title_position") {
    match v.trim().to_lowercase().as_str() {
        "top" => conf.switch_windows_title_position = TitlePosition::Top,
        "overlay" => conf.switch_windows_title_position = TitlePosition::Overlay,
        "bottom" | _ => conf.switch_windows_title_position = TitlePosition::Bottom,
    }
}
if let Some(v) = section.get("show_app_icon").and_then(Config::to_bool) {
    conf.switch_windows_show_app_icon = v;
}
if let Some(v) = section.get("enable_quick_actions").and_then(Config::to_bool) {
    conf.switch_windows_enable_quick_actions = v;
}
```

Add setting entries in `window-switcher.ini`:
```ini
show_title = yes
title_position = bottom
show_app_icon = yes
enable_quick_actions = yes
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --target=x86_64-pc-windows-msvc test_config_title_position_parse`
Expected: PASS

- [ ] **Step 5: Check formatting and lints**

Run: `cargo clippy --all --all-targets`
Expected: 0 warnings.

---

### Task 2: Data Model & Overlay State (`src/app.rs`)

**Files:**
- Modify: `src/app.rs:48-90`, `src/app.rs:394-475`, `src/app.rs:585-597`

- [ ] **Step 1: Define `WindowOverlayItem` struct**

In `src/app.rs`:
```rust
#[derive(Debug, Clone)]
pub struct WindowOverlayItem {
    pub hwnd: HWND,
    pub title: String,
    pub hicon: Option<HICON>,
}
```

- [ ] **Step 2: Update `SwitchWindowsState` fields**

Update `SwitchWindowsState`:
```rust
#[derive(Debug)]
struct SwitchWindowsState {
    cache: Option<(String, HWND, usize, Vec<isize>)>,
    modifier_released: bool,
    overlay_windows: Option<Vec<WindowOverlayItem>>,
    overlay_index: usize,
    hovered_close_index: Option<usize>,
}
```

- [ ] **Step 3: Update `switch_windows_overlay` to populate `WindowOverlayItem`**

In `switch_windows_overlay` (`src/app.rs`):
Build items with app icons cached via `self.cached_icons`:
```rust
let overlay: Vec<WindowOverlayItem> = window_list
    .iter()
    .map(|(h, title)| {
        let hicon = self.cached_icons.get(&module_path).copied();
        WindowOverlayItem {
            hwnd: *h,
            title: title.clone(),
            hicon,
        }
    })
    .collect();
```

- [ ] **Step 4: Implement quick-action window closing helper `close_overlay_window(index)`**

```rust
fn close_overlay_window(&mut self, index: usize) {
    if let Some(overlay) = self.switch_windows_state.overlay_windows.as_mut() {
        if index < overlay.len() {
            let item = overlay.remove(index);
            unsafe {
                let _ = PostMessageW(Some(item.hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
            if overlay.is_empty() {
                self.switch_windows_state.overlay_windows = None;
                self.painter.unpaint_switch_windows();
            } else {
                if self.switch_windows_state.overlay_index >= overlay.len() {
                    self.switch_windows_state.overlay_index = overlay.len() - 1;
                }
                let idx = self.switch_windows_state.overlay_index;
                let windows = self.switch_windows_state.overlay_windows.as_ref().unwrap();
                self.painter.paint_switch_windows(windows, idx, &self.config, self.switch_windows_state.hovered_close_index);
            }
        }
    }
}
```

- [ ] **Step 5: Verify build**

Run: `cargo check --target=x86_64-pc-windows-msvc`
Expected: PASS (or expected painter parameters adjustment).

---

### Task 3: Layout Geometry & Hit Testing (`src/painter.rs`)

**Files:**
- Modify: `src/painter.rs:260-440`, `src/painter.rs:657-703`

- [ ] **Step 1: Define `ThumbnailHitTarget` enum**

In `src/painter.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbnailHitTarget {
    Card(usize),
    CloseButton(usize),
}
```

- [ ] **Step 2: Update `ThumbCoordinate` for dynamic title height**

Update `ThumbCoordinate::new` to accept `config: &Config`:
```rust
let title_height = if config.switch_windows_show_title
    && config.switch_windows_title_position != TitlePosition::Overlay
{
    (24.0 * dpi_scale) as i32
} else {
    0
};

let thumb_item_height = thumb_height + thumb_border * 2 + title_height;
```

- [ ] **Step 3: Implement hit testing function `hit_test_switch_windows`**

```rust
pub fn hit_test_switch_windows(&self, windows_len: usize, config: &Config) -> Option<ThumbnailHitTarget> {
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
                let close_y = cy + thumb_border;
                if xpos >= close_x && xpos < close_x + close_size && ypos >= close_y && ypos < close_y + close_size {
                    return Some(ThumbnailHitTarget::CloseButton(i));
                }
            }
            return Some(ThumbnailHitTarget::Card(i));
        }
    }
    None
}
```

---

### Task 4: GDI+ Card & Title Rendering Engine (`src/painter.rs`)

**Files:**
- Modify: `src/painter.rs:260-390`

- [ ] **Step 1: Implement GDI+ title text drawing (`GdipDrawString`)**

In `paint_switch_windows` (`src/painter.rs`), render title text according to `config.switch_windows_title_position`:
- Create GDI+ `FontFamily` ("Segoe UI") and `Font`.
- Create `StringFormat` with `StringAlignmentCenter` and `StringTrimmingEllipsisCharacter`.
- Construct bounding rectangle `RECTF`.
- If `TitlePosition::Overlay`: draw semi-transparent background rect over thumbnail bottom, then render text in white.
- If `TitlePosition::Bottom` or `Top`: draw text in theme color (`fg_color`).

- [ ] **Step 2: Implement App Icon Badge drawing**

If `config.switch_windows_show_app_icon` is true and `item.hicon` is `Some(hicon)`:
- Call `DrawIconEx(hdc_mem, icon_x, icon_y, hicon, icon_size, icon_size, 0, None, DI_NORMAL)`.

- [ ] **Step 3: Implement Quick Action Close Button drawing**

If `config.switch_windows_enable_quick_actions` is true:
- Draw circle at top-right of thumbnail card.
- If `hovered_close_index == Some(i)`: fill circle with solid red brush (`0xffe81123`).
- Draw 'X' text or lines inside circle.

---

### Task 5: Mouse Input Integration & Verification

**Files:**
- Modify: `src/app.rs:267-275`
- Test: `cargo test --target=x86_64-pc-windows-msvc --all`
- Lint: `cargo clippy --all --all-targets`

- [ ] **Step 1: Wire `WM_LBUTTONUP`, `WM_MBUTTONUP`, and `WM_MOUSEMOVE` in `handle_message`**

In `handle_message` (`src/app.rs`):
- On `WM_LBUTTONUP`: Call `hit_test_switch_windows`. Switch window if `Card(i)`, close window if `CloseButton(i)`.
- On `WM_MBUTTONUP`: If `enable_quick_actions` is true, close window on `Card(i)` or `CloseButton(i)`.
- On `WM_MOUSEMOVE`: Update `hovered_close_index` and trigger repaint if hover state changes.

- [ ] **Step 2: Run all unit tests**

Run: `cargo test --target=x86_64-pc-windows-msvc --all`
Expected: All tests PASS.

- [ ] **Step 3: Run Clippy check**

Run: `cargo clippy --all --all-targets`
Expected: 0 warnings (deny warnings enforced).

- [ ] **Step 4: Check formatting**

Run: `cargo fmt --all --check`
Expected: Formatting check PASS.
