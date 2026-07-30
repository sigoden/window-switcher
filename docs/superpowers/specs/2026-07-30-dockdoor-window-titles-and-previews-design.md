# DockDoor-Style Window Titles, App Icons, and Quick Actions Preview Design

## 1. Overview & Requirements

This document specifies the design for bringing macOS DockDoor-style preview capabilities to `window-switcher`. 

### Key Features
1. **Window Titles on Preview Cards**: Render real-time window titles on thumbnail cards with configurable positioning (`bottom`, `top`, or `overlay`).
2. **App Icon Badge**: Display small app icons on preview cards for quick visual app identification.
3. **Quick Action Window Closing**: Close individual windows directly from the overlay via middle-clicking a card or clicking a close ('X') button.
4. **User Configuration**: All preview options are toggleable and configurable in `window-switcher.ini`.

---

## 2. Configuration & Default Settings (`window-switcher.ini` & `src/config.rs`)

### `window-switcher.ini` Additions
New options added under the `[switch-windows]` section:

```ini
[switch-windows]
# Show window title on thumbnail preview cards (yes/no)
show_title = yes

# Position of the window title: bottom, top, or overlay
title_position = bottom

# Show app icon badge on each preview card (yes/no)
show_app_icon = yes

# Enable quick actions (middle click or hover 'X' button to close window) (yes/no)
enable_quick_actions = yes
```

### `src/config.rs` Data Structures
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TitlePosition {
    #[default]
    Bottom,
    Top,
    Overlay,
}
```

New fields in `Config`:
- `switch_windows_show_title: bool` (default: `true`)
- `switch_windows_title_position: TitlePosition` (default: `TitlePosition::Bottom`)
- `switch_windows_show_app_icon: bool` (default: `true`)
- `switch_windows_enable_quick_actions: bool` (default: `true`)

---

## 3. Data Flow & State Management (`src/app.rs`)

### `WindowOverlayItem` Struct
```rust
#[derive(Debug, Clone)]
pub struct WindowOverlayItem {
    pub hwnd: HWND,
    pub title: String,
    pub hicon: Option<HICON>,
}
```

### `SwitchWindowsState` Updates
```rust
struct SwitchWindowsState {
    cache: Option<(String, HWND, usize, Vec<isize>)>,
    modifier_released: bool,
    overlay_windows: Option<Vec<WindowOverlayItem>>,
    overlay_index: usize,
    hovered_close_index: Option<usize>,
}
```

### Window Close Logic
When closing a window from quick actions:
1. `PostMessageW(Some(item.hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));`
2. Remove the closed item from `overlay_windows`.
3. If `overlay_windows` is empty, invoke `painter.unpaint_switch_windows()`.
4. Otherwise, clamp `overlay_index` and re-render `painter.paint_switch_windows(...)`.

---

## 4. Rendering Engine (`src/painter.rs`)

### Dynamic Card Dimensions (`ThumbCoordinate`)
- `title_height`: Calculated based on DPI and whether `show_title` is true and `title_position` is `Bottom` or `Top`.
- Thumbnail position offsets shift based on `Top` vs `Bottom` title configuration.

### GDI+ Text & Icon Rendering
- **Title Text**: Drawn via GDI+ `GdipDrawString` using Segoe UI font, `StringAlignmentCenter`, and `StringTrimmingEllipsisCharacter`.
- **Overlay Title Backdrop**: For `TitlePosition::Overlay`, a translucent dark/light rounded rectangle is rendered over the bottom of the thumbnail before text is drawn.
- **App Icon Badge**: 20x20px app icon drawn at top-left using `DrawIconEx`.
- **Close Button**: 18x18px circle with an 'X' at top-right. Highlighted red (`0xffe81123`) when hovered.

---

## 5. Mouse Interaction & Hit Testing (`src/app.rs` & `src/painter.rs`)

### `ThumbnailHitTarget` Enum
```rust
pub enum ThumbnailHitTarget {
    Card(usize),
    CloseButton(usize),
}
```

### Hit Testing & Event Handlers
- `WM_LBUTTONUP`:
  - `CloseButton(i)` -> Close window `i`.
  - `Card(i)` -> Switch focus to window `i`.
- `WM_MBUTTONUP`:
  - If `enable_quick_actions` is `true` -> Close window `i`.
- `WM_MOUSEMOVE`:
  - Update `hovered_close_index` and trigger repaint if hover state changed.

---

## 6. Testing & Verification Plan

### Unit Tests
- Config loader tests verifying INI parsing for `show_title`, `title_position`, `show_app_icon`, and `enable_quick_actions`.
- Fallback tests for invalid or missing INI options.
- Layout coordinate tests verifying card bounds calculations.

### Linting & Formatting
- `cargo clippy --all --all-targets`
- `cargo fmt --all --check`
