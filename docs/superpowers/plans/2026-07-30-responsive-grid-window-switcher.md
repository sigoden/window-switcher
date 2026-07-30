# Responsive Grid Same-App Window Switcher Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the same-app `Alt+Backtick` thumbnail strip with a responsive, cached, aspect-correct Fluent grid that remains readable and responsive with two to ten windows.

**Architecture:** Add a pure `window_grid` module for DPI-aware paging, balanced layout, aspect fitting, traversal, and hit testing. Make `thumbnail` return owned cached bitmaps, keep overlay-session data in `app`, and restrict `painter` to rendering the prepared model and layout. Extend the keyboard hook with explicit same-app cancellation while preserving release-to-activate behavior.

**Tech Stack:** Rust 2021, Win32 APIs through `windows` 0.61.3, GDI/GDI+, layered windows, `rust-ini`, in-module Rust unit tests.

## Global Constraints

- Build and test target is always `x86_64-pc-windows-msvc`.
- CI treats every warning as an error with `RUSTFLAGS=--deny warnings`.
- Preserve `#![windows_subsystem = "windows"]` in `src/main.rs`.
- Do not add runtime dependencies.
- The responsive grid replaces the old strip; there is no legacy layout mode.
- Titles and the shared app header are mandatory, not configuration switches.
- Preview bitmaps are captured once per overlay session and released when it ends.
- Preview content must preserve its source aspect ratio.
- The overlay must use the foreground window's monitor work area, not cursor position or full monitor bounds.
- Pages contain at most ten windows, arranged in no more than two rows and five columns.

---

## File Structure Map

- Create `src/window_grid.rs`: platform-independent geometry, paging, traversal, aspect-fit, and hit-test logic plus unit tests.
- Modify `src/lib.rs`: register the new internal module.
- Modify `src/config.rs`: retain only the supported grid settings after the old painter is removed.
- Replace `window-switcher.ini`: publish the clean supported schema.
- Modify `src/utils/window.rs`: return the foreground window monitor's work area.
- Modify `src/utils/mod.rs`: export the new monitor helper.
- Modify `src/thumbnail.rs`: introduce an owned bitmap and aspect-correct cached capture.
- Create `src/window_overlay.rs`: own overlay-session data, cached thumbnails, app identity, selection, and hover state.
- Modify `src/app.rs`: construct and mutate overlay sessions, handle cancellation, and close/reflow behavior.
- Modify `src/painter.rs`: replace same-app strip rendering and hit testing with grid rendering.
- Modify `src/keyboard.rs`: send an explicit same-app cancel message on Escape.
- Modify `README.md`: document the new same-app preview behavior and current INI schema.

---

### Task 1: Establish the Replacement Preview Configuration

**Files:**
- Modify: `src/config.rs:12-145`
- Modify: `src/config.rs:420-470`
- Replace: `window-switcher.ini`

**Interfaces:**
- Consumes: existing `Config::load(&Ini) -> anyhow::Result<Config>`.
- Produces: a bundled INI containing only the supported grid settings. Rust
  fields used by the old painter remain temporarily and are removed in Task 8
  after that painter no longer exists.

- [ ] **Step 1: Replace the obsolete config test with a clean-schema coverage test**

Replace `test_config_title_position_parse` with:

```rust
#[test]
fn test_switch_windows_preview_config() {
    let ini = Ini::load_from_str(
        r#"
[switch-windows]
thumbnail_preview = no
enable_quick_actions = no
"#,
    )
    .unwrap();

    let config = Config::load(&ini).unwrap();
    assert!(!config.switch_windows_thumbnail_preview);
    assert!(!config.switch_windows_enable_quick_actions);
}
```

- [ ] **Step 2: Run the focused test before changing production code**

Run:

```bash
cargo test --target=x86_64-pc-windows-msvc test_switch_windows_preview_config
```

Expected: PASS against the old parser, establishing coverage for the two
settings retained by the replacement schema.

- [ ] **Step 3: Replace the example INI**

Replace `window-switcher.ini` with:

```ini
# Whether to show the notification-area icon.
trayicon = yes

[switch-windows]
# Switch between windows belonging to the foreground app.
hotkey = alt+`
blacklist =
ignore_minimal = no
thumbnail_preview = yes
enable_quick_actions = yes
only_current_desktop = auto

[switch-apps]
# Optional app-level switcher.
enable = no
hotkey = alt+tab
ignore_minimal = no
override_icons =
only_current_desktop = auto

[log]
level = info
path =
```

- [ ] **Step 4: Run config tests and a compile check**

Run:

```bash
cargo test --target=x86_64-pc-windows-msvc config::tests
cargo check --target=x86_64-pc-windows-msvc
```

Expected: config tests and `cargo check` PASS.

- [ ] **Step 5: Commit the configuration cleanup**

```bash
git add src/config.rs window-switcher.ini
git commit -m "refactor: simplify window preview configuration"
```

---

### Task 2: Add the Pure Responsive Grid Engine

**Files:**
- Create: `src/window_grid.rs`
- Modify: `src/lib.rs:8-15`

**Interfaces:**
- Consumes: integer monitor work-area geometry, DPI scale, total item count, and selected global index.
- Produces:
  - `RectI { x: i32, y: i32, width: i32, height: i32 }`
  - `PointI { x: i32, y: i32 }`
  - `CardLayout { global_index: usize, card: RectI, preview: RectI, title: RectI, close: RectI }`
  - `WindowGridLayout { bounds: RectI, surface: RectI, header: RectI, cards: Vec<CardLayout>, page_start: usize, total_items: usize }`
  - `GridHitTarget::{Card(usize), CloseButton(usize)}`
  - `WindowGridLayout::new(work_area: RectI, dpi_scale: f64, total_items: usize, selected: usize) -> Self`
  - `WindowGridLayout::row_lengths(&self) -> Vec<usize>`
  - `WindowGridLayout::hit_test(&self, point: PointI, quick_actions: bool) -> Option<GridHitTarget>`
  - `advance_index(current: usize, len: usize, reverse: bool) -> usize`
  - `fit_aspect(source_width: i32, source_height: i32, frame: RectI) -> RectI`

- [ ] **Step 1: Register the module and write failing geometry tests**

Add `mod window_grid;` to `src/lib.rs`.

Create `src/window_grid.rs` with the public data types above and these tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const WORK_AREA: RectI = RectI::new(0, 0, 1920, 1080);

    #[test]
    fn balances_six_through_ten_items() {
        let expected = [(6, vec![3, 3]), (7, vec![4, 3]), (8, vec![4, 4]),
            (9, vec![5, 4]), (10, vec![5, 5])];

        for (count, row_lengths) in expected {
            let layout = WindowGridLayout::new(WORK_AREA, 1.0, count, 1);
            assert_eq!(layout.row_lengths(), row_lengths);
            assert!(WORK_AREA.contains_rect(layout.bounds));
        }
    }

    #[test]
    fn pages_after_ten_items() {
        let first = WindowGridLayout::new(WORK_AREA, 1.0, 14, 9);
        let second = WindowGridLayout::new(WORK_AREA, 1.0, 14, 10);
        assert_eq!(first.page_start, 0);
        assert_eq!(first.cards.len(), 10);
        assert_eq!(second.page_start, 10);
        assert_eq!(second.cards.iter().map(|c| c.global_index).collect::<Vec<_>>(),
            vec![10, 11, 12, 13]);
        assert_eq!(first.cards[0].preview.width, second.cards[0].preview.width);
        assert_eq!(first.cards[0].preview.height, second.cards[0].preview.height);
    }

    #[test]
    fn stays_inside_work_area_at_supported_scales() {
        for dpi in [1.0, 1.25, 1.5, 2.0] {
            let layout = WindowGridLayout::new(WORK_AREA, dpi, 10, 1);
            assert!(WORK_AREA.contains_rect(layout.bounds));
        }
    }

    #[test]
    fn traverses_forward_reverse_and_wraps() {
        assert_eq!(advance_index(0, 8, false), 1);
        assert_eq!(advance_index(7, 8, false), 0);
        assert_eq!(advance_index(0, 8, true), 7);
        assert_eq!(advance_index(4, 8, true), 3);
        assert_eq!(advance_index(3, 0, false), 0);
    }

    #[test]
    fn aspect_fit_preserves_shape_and_centers() {
        let frame = RectI::new(10, 20, 300, 200);
        assert_eq!(fit_aspect(1600, 900, frame), RectI::new(10, 36, 300, 168));
        assert_eq!(fit_aspect(900, 1600, frame), RectI::new(104, 20, 112, 200));
        assert_eq!(fit_aspect(1000, 1000, frame), RectI::new(60, 20, 200, 200));
        assert_eq!(fit_aspect(0, 1000, frame), frame);
    }

    #[test]
    fn hit_test_distinguishes_card_and_close_button() {
        let layout = WindowGridLayout::new(WORK_AREA, 1.0, 6, 1);
        let card = layout.cards[0];
        assert_eq!(
            layout.hit_test(card.close.center(), true),
            Some(GridHitTarget::CloseButton(0))
        );
        assert_eq!(
            layout.hit_test(card.preview.center(), true),
            Some(GridHitTarget::Card(0))
        );
        assert_eq!(
            layout.hit_test(card.close.center(), false),
            Some(GridHitTarget::Card(0))
        );
    }
}
```

- [ ] **Step 2: Run the new tests and verify they fail**

Run:

```bash
cargo test --target=x86_64-pc-windows-msvc window_grid::tests
```

Expected: compilation fails because the layout types and functions are not yet
implemented.

- [ ] **Step 3: Implement geometry, paging, and traversal**

Use these base metrics before DPI/work-area fitting:

```rust
const PAGE_SIZE: usize = 10;
const MAX_COLUMNS: usize = 5;
const CARD_WIDTH: i32 = 256;
const PREVIEW_HEIGHT: i32 = 144;
const TITLE_HEIGHT: i32 = 28;
const HEADER_HEIGHT: i32 = 40;
const CARD_PADDING: i32 = 8;
const OUTER_PADDING: i32 = 14;
const GAP: i32 = 10;
const CLOSE_SIZE: i32 = 20;
const SHADOW_MARGIN: i32 = 12;
const WORK_AREA_PERCENT: i32 = 90;
```

Implement row sizes with:

```rust
fn balanced_rows(count: usize) -> Vec<usize> {
    if count <= MAX_COLUMNS {
        vec![count]
    } else {
        let first = count.div_ceil(2);
        vec![first, count - first]
    }
}
```

Calculate a nominal overlay from the largest row and row count. Scale all
metrics by:

```rust
let width_limit = work_area.width * WORK_AREA_PERCENT / 100;
let height_limit = work_area.height * WORK_AREA_PERCENT / 100;
let scale = dpi_scale
    .min(width_limit as f64 / nominal_width as f64)
    .min(height_limit as f64 / nominal_height as f64)
    .max(0.5);
```

If the `0.5` floor would exceed the work area, use the smaller width/height
ratio instead. Center the overlay in `work_area`, center each row independently,
and map visible cards to `page_start + local_index`. `bounds` includes the
DPI-scaled shadow margin; `surface` is the inset rounded panel containing the
header and cards.

For `total_items > PAGE_SIZE`, calculate nominal bounds and scale from a full
five-by-two page even when the final page contains fewer items. Center the
shorter final-page rows within that stable surface. This keeps preview
dimensions identical across pages so cached bitmaps are never enlarged after a
page transition.

Implement `fit_aspect` with integer cross multiplication and center the fitted
rectangle in `frame`. Implement `RectI::contains`, `contains_rect`, and `center`
so hit testing has no Win32 dependency.

- [ ] **Step 4: Run the grid tests**

Run:

```bash
cargo test --target=x86_64-pc-windows-msvc window_grid::tests
```

Expected: all grid tests PASS.

- [ ] **Step 5: Commit the pure grid engine**

```bash
git add src/lib.rs src/window_grid.rs
git commit -m "feat: add responsive window grid layout"
```

---

### Task 3: Use the Foreground Monitor Work Area

**Files:**
- Modify: `src/utils/window.rs:80-100`
- Modify: `src/utils/mod.rs`
- Modify: `src/painter.rs:1-20`
- Modify: `src/painter.rs:850-865`
- Test: `src/window_grid.rs`

**Interfaces:**
- Consumes: `HWND` for the foreground window.
- Produces: `pub fn get_monitor_work_area(hwnd: HWND) -> RECT`, using `MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST)` and `MONITORINFO::rcWork`.
- Produces: `pub fn get_dpi_scale(hwnd: HWND) -> f64`, using
  `GetDpiForWindow(hwnd)` with a `1.0` fallback when Windows returns zero.

- [ ] **Step 1: Add a conversion test for Win32 rectangles**

Add to `window_grid`:

```rust
impl From<windows::Win32::Foundation::RECT> for RectI {
    fn from(rect: windows::Win32::Foundation::RECT) -> Self {
        Self::new(
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
        )
    }
}
```

Add:

```rust
#[test]
fn converts_win32_rect_to_origin_and_size() {
    let rect = windows::Win32::Foundation::RECT {
        left: -1920,
        top: 40,
        right: 0,
        bottom: 1080,
    };
    assert_eq!(RectI::from(rect), RectI::new(-1920, 40, 1920, 1040));
}
```

- [ ] **Step 2: Run the focused test**

Run:

```bash
cargo test --target=x86_64-pc-windows-msvc converts_win32_rect_to_origin_and_size
```

Expected: PASS after adding the conversion; this locks the negative-monitor
coordinate behavior before the Win32 helper is changed.

- [ ] **Step 3: Add the foreground work-area helper**

Add alongside the existing app-switcher cursor helper:

```rust
pub fn get_monitor_work_area(hwnd: HWND) -> RECT {
    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            info.rcWork
        } else {
            RECT::default()
        }
    }
}
```

Add the `MonitorFromWindow` import and export the helper through
`src/utils/mod.rs`. Keep `get_moinitor_rect()` unchanged because the optional
app-level icon switcher still consumes it; renaming that existing API is outside
this feature.

Move the existing `get_dpi_scale` implementation from `painter.rs` into
`src/utils/window.rs`, make it public, and export it from `src/utils/mod.rs`.
Update `painter.rs` to import the helper. This gives layout construction and
painting one DPI source.

- [ ] **Step 4: Compile the utility change**

Run:

```bash
cargo check --target=x86_64-pc-windows-msvc
```

Expected: the entire crate compiles; the new helper is consumed in Task 6.

- [ ] **Step 5: Commit monitor placement**

```bash
git add src/utils/window.rs src/utils/mod.rs src/painter.rs src/window_grid.rs
git commit -m "fix: place window grid in foreground work area"
```

---

### Task 4: Cache Aspect-Correct Owned Thumbnails

**Files:**
- Modify: `src/thumbnail.rs`
- Test: `src/thumbnail.rs`

**Interfaces:**
- Consumes: `HWND`, target preview width, and target preview height.
- Produces:
  - `pub struct OwnedBitmap(HBITMAP)` with `handle(&self) -> HBITMAP` and `Drop`.
  - `pub fn capture_window_thumbnail_cached(hwnd: HWND, target_width: i32, target_height: i32) -> Option<OwnedBitmap>`.
- Uses: `window_grid::fit_aspect` to place source pixels without distortion.

- [ ] **Step 1: Add ownership and fallback tests**

Extract target rectangle selection into:

```rust
fn capture_destination(source_width: i32, source_height: i32, width: i32, height: i32) -> RectI {
    fit_aspect(
        source_width,
        source_height,
        RectI::new(0, 0, width, height),
    )
}
```

Add:

```rust
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
}
```

- [ ] **Step 2: Run the tests and verify they fail**

Run:

```bash
cargo test --target=x86_64-pc-windows-msvc thumbnail::tests
```

Expected: compilation fails because `capture_destination` does not exist.

- [ ] **Step 3: Implement owned, aspect-correct capture**

Define:

```rust
pub struct OwnedBitmap(HBITMAP);

impl OwnedBitmap {
    pub fn handle(&self) -> HBITMAP {
        self.0
    }

    fn into_raw(mut self) -> HBITMAP {
        std::mem::replace(&mut self.0, HBITMAP::default())
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
```

Implement `capture_window_thumbnail_cached` by acquiring/releasing its own
screen DC with `GetDC(None)` and
`ReleaseDC(None, hdc_screen)`. After `PrintWindow` succeeds:

1. Create a target bitmap of the requested dimensions.
2. Fill the entire target with a neutral black surface.
3. Calculate `destination = capture_destination(src_w, src_h, target_width, target_height)`.
4. `StretchBlt` into that destination using `HALFTONE`.
5. Delete source DC/bitmap objects.
6. Return `Some(OwnedBitmap(bitmap_target))`.

Detect an unusable black capture with:

```rust
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
```

Return `None` for invalid source dimensions, failed `PrintWindow`, a blank
capture, or invalid target dimensions. The new cached path leaves icon fallback
to the grid painter. This intentionally treats a completely black sampled
window as unavailable rather than displaying a broken preview.

Keep the old `capture_window_thumbnail(hwnd, hdc_screen, width, height) ->
HBITMAP` temporarily as a compatibility wrapper for the strip painter. It calls
`capture_window_thumbnail_cached(...).map(OwnedBitmap::into_raw)` and uses the
existing icon fallback when cached capture returns `None`. Task 6 removes this
wrapper with the strip renderer. This keeps Task 4 compiling independently.

- [ ] **Step 4: Run thumbnail and grid tests**

Run:

```bash
cargo test --target=x86_64-pc-windows-msvc thumbnail::tests
cargo test --target=x86_64-pc-windows-msvc window_grid::tests
```

Expected: all tests PASS.

- [ ] **Step 5: Commit thumbnail ownership**

```bash
git add src/thumbnail.rs
git commit -m "feat: cache aspect-correct window thumbnails"
```

---

### Task 5: Replace the Strip Painter with the Fluent Grid Painter

**Files:**
- Create: `src/window_overlay.rs`
- Modify: `src/lib.rs:8-16`
- Modify: `src/painter.rs:1-40`
- Modify: `src/painter.rs:255-650`
- Test: `src/window_grid.rs`

**Interfaces:**
- Consumes:
  - `WindowOverlaySession`
  - `quick_actions: bool`
- Produces:
  - `WindowOverlayItem { hwnd, title, thumbnail }`
  - `WindowOverlaySession { app_name, app_icon, original_hwnd, items, selected, hovered, hovered_close, layout }`
  - `GdiAAPainter::paint_window_grid(&WindowOverlaySession, bool)`
  - `GdiAAPainter::unpaint_window_grid()`
- Temporarily retains the old strip methods until `app.rs` switches callers in
  Task 6, keeping this task independently compilable.

- [ ] **Step 1: Define the session-owned overlay model**

Register `mod window_overlay;` in `src/lib.rs` and create:

```rust
use crate::{thumbnail::OwnedBitmap, window_grid::WindowGridLayout};
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{HICON, IsWindow},
};

pub(crate) struct WindowOverlayItem {
    pub hwnd: HWND,
    pub title: String,
    pub thumbnail: Option<OwnedBitmap>,
}

pub(crate) struct WindowOverlaySession {
    pub app_name: String,
    pub app_icon: HICON,
    pub original_hwnd: HWND,
    pub items: Vec<WindowOverlayItem>,
    pub selected: usize,
    pub hovered: Option<usize>,
    pub hovered_close: Option<usize>,
    pub layout: WindowGridLayout,
}

impl WindowOverlaySession {
    pub fn prune_invalid(&mut self) {
        self.items
            .retain(|item| unsafe { IsWindow(Some(item.hwnd)).as_bool() });
        if !self.items.is_empty() {
            self.selected = self.selected.min(self.items.len() - 1);
        }
    }
}
```

- [ ] **Step 2: Add layout assertions for painter-visible page indices**

Add:

```rust
#[test]
fn selected_item_is_always_on_visible_page() {
    for selected in 0..24 {
        let layout = WindowGridLayout::new(WORK_AREA, 1.0, 24, selected);
        assert!(layout.cards.iter().any(|card| card.global_index == selected));
        assert!(layout.cards.len() <= 10);
    }
}
```

- [ ] **Step 3: Run the focused test**

Run:

```bash
cargo test --target=x86_64-pc-windows-msvc selected_item_is_always_on_visible_page
```

Expected: PASS against Task 2's paging implementation.

- [ ] **Step 4: Introduce grid-specific paint helpers**

Keep the existing app-level icon painter unchanged. Add focused private helpers
in `painter.rs`:

```rust
unsafe fn draw_text(
    graphics: *mut GpGraphics,
    text: &str,
    rect: RectI,
    font_size: f32,
    color: u32,
    trimming: StringTrimming,
);

unsafe fn draw_bitmap(
    graphics: *mut GpGraphics,
    bitmap: HBITMAP,
    rect: RectI,
);

unsafe fn draw_app_icon_fallback(
    hdc: HDC,
    icon: HICON,
    rect: RectI,
);
```

Each helper must create and destroy every GDI+ font, format, image, brush, and
pen it owns before returning.

- [ ] **Step 5: Implement `paint_window_grid`**

Use this signature:

```rust
pub fn paint_window_grid(
    &mut self,
    session: &WindowOverlaySession,
    quick_actions: bool,
)
```

Rendering order:

1. Allocate the layered-window backing bitmap at
   `session.layout.bounds.width` × `session.layout.bounds.height`.
2. Translate the absolute rectangles from `session.layout` into coordinates
   relative to `session.layout.bounds`.
3. Draw four rounded, low-alpha black layers between `session.layout.bounds`
   and `session.layout.surface` to form a soft shadow, then draw the
   alpha-themed rounded surface.
4. Draw `session.app_icon` and `session.app_name` in `session.layout.header`.
5. Right-align
   `format!("{} of {}", session.selected + 1, session.items.len())` in the
   header.
6. For every `session.layout.cards` entry, draw the card background and
   selected/hover state.
7. Draw the cached thumbnail with `GdipDrawImageRect`; if absent, draw the app
   icon centered in the preview.
8. Draw `Untitled` when the item's `title.trim().is_empty()`, otherwise its title,
   using Segoe UI, ellipsis trimming, and one line.
9. Draw the close button only when quick actions are enabled and the card is
   selected or hovered. Use a red fill only when
   `session.hovered_close == Some(card.global_index)`.
10. Call `UpdateLayeredWindow`, clean up all GDI objects, and show the overlay
    without calling `SetFocus`; the global keyboard hook does not require focus,
    and preserving focus keeps Escape cancellation deterministic.

Use Windows theme colors for the surface and `GetSysColor(COLOR_HIGHLIGHT)` for
the selected outline. Keep a fixed high-contrast fallback color if
`GetSysColor` returns zero.

- [ ] **Step 6: Keep the old strip entry points temporarily**

Do not delete `paint_switch_windows`, `unpaint_switch_windows`,
`hit_test_switch_windows`, `ThumbCoordinate`, or `ThumbnailHitTarget` in this
task. `app.rs` still calls them. Task 6 removes all of them immediately after
switching the application to `paint_window_grid`.

- [ ] **Step 7: Format and compile the painter**

Run:

```bash
cargo fmt --all
cargo check --target=x86_64-pc-windows-msvc
```

Expected: the entire crate compiles because both old and new painter entry
points exist.

- [ ] **Step 8: Commit the renderer**

```bash
git add src/lib.rs src/window_overlay.rs src/painter.rs src/window_grid.rs
git commit -m "feat: render same-app windows as a Fluent grid"
```

---

### Task 6: Integrate Overlay Session State, Paging, Hover, and Close

**Files:**
- Modify: `src/app.rs:1-110`
- Modify: `src/app.rs:230-340`
- Modify: `src/app.rs:430-590`
- Modify: `src/app.rs:680-710`
- Modify: `src/painter.rs:255-650`
- Modify: `src/painter.rs:900-960`
- Modify: `src/thumbnail.rs`

**Interfaces:**
- Consumes:
  - `WindowGridLayout::new`
  - `WindowGridLayout::hit_test`
  - `advance_index`
  - `capture_window_thumbnail_cached`
  - `GdiAAPainter::paint_window_grid`
- Produces:
  - `App::cancel_switch_windows()`
  - `App::repaint_window_grid()`
  - `App::hit_test_window_grid() -> Option<GridHitTarget>`

- [ ] **Step 1: Replace old application state with the session model**

Import `WindowOverlayItem` and `WindowOverlaySession` from
`crate::window_overlay`. Replace the old overlay fields with:

```rust
struct SwitchWindowsState {
    cache: Option<(String, HWND, usize, Vec<isize>)>,
    modifier_released: bool,
    overlay: Option<WindowOverlaySession>,
}
```

Remove the old `WindowOverlayItem` definition from `app.rs`. The session types
remain non-cloneable so each `OwnedBitmap` has one owner.

- [ ] **Step 2: Build a session once when the overlay opens**

In `switch_windows_overlay`:

1. Keep the current module lookup and MRU window list.
2. Resolve `app_name` with:

```rust
let app_name = std::path::Path::new(&module_path)
    .file_stem()
    .and_then(|name| name.to_str())
    .filter(|name| !name.is_empty())
    .unwrap_or("Application")
    .to_string();
```

3. Resolve the shared icon once through `cached_icons`.
4. Set initial selection to the last item for reverse, otherwise index `1`.
5. Convert `get_monitor_work_area(hwnd)` into `RectI`.
6. Build
   `WindowGridLayout::new(work_area, get_dpi_scale(self.hwnd), len, selected)`.
7. Capture each bitmap exactly once with
   `capture_window_thumbnail_cached` using the card preview dimensions from the
   first page. Since all pages use the same metrics, use
   `layout.cards[0].preview.width` and `.height`.
8. Store the session and call `repaint_window_grid`.

Normalize titles during construction:

```rust
let title = if title.trim().is_empty() {
    "Untitled".to_string()
} else {
    title.clone()
};
```

- [ ] **Step 3: Reuse cached items while cycling**

When a session already exists:

```rust
session.selected = advance_index(session.selected, session.items.len(), reverse);
session.layout = WindowGridLayout::new(
    work_area,
    dpi_scale,
    session.items.len(),
    session.selected,
);
```

Then repaint without calling `capture_window_thumbnail_cached`. This is the
central performance requirement.

- [ ] **Step 4: Centralize repainting and hit testing**

Add:

```rust
fn repaint_window_grid(&mut self) {
    if let Some(session) = self.switch_windows_state.overlay.as_ref() {
        self.painter.paint_window_grid(
            session,
            self.config.switch_windows_enable_quick_actions,
        );
    }
}

fn hit_test_window_grid(&self) -> Option<GridHitTarget> {
    let session = self.switch_windows_state.overlay.as_ref()?;
    let mut cursor = POINT::default();
    unsafe { GetCursorPos(&mut cursor).ok()? };
    session.layout.hit_test(
        PointI { x: cursor.x, y: cursor.y },
        self.config.switch_windows_enable_quick_actions,
    )
}
```

- [ ] **Step 5: Update pointer behavior**

Implement:

- `WM_MOUSEMOVE`: `Card(i)` sets `hovered = Some(i)` and
  `hovered_close = None`; `CloseButton(i)` sets both to `Some(i)`; no hit clears
  both. Repaint only when either value changes.
- `WM_MOUSELEAVE`: clear both hover fields and repaint once.
- `WM_LBUTTONUP`: close a close-button target; activate a card target.
- `WM_MBUTTONUP`: close either target when quick actions are enabled.

Hover must never change `selected`.

- [ ] **Step 6: Update activation, closing, and resource release**

`do_switch_windows` takes the session, activates `items[selected].hwnd`, and
hides the painter. Taking the session drops every cached `OwnedBitmap`.

`close_overlay_window(index)`:

1. Remove the item and post `WM_CLOSE`.
2. If no items remain, take the session and hide the grid.
3. Otherwise select `index.min(items.len() - 1)`.
4. Clear hover state.
5. Rebuild the layout for the new count and selection.
6. Repaint without recapturing remaining items.

Before activation and on repaint, remove items whose handles fail `IsWindow`.
Apply the same clamp/rebuild steps after pruning.

- [ ] **Step 7: Remove the strip renderer and compatibility capture**

After all `app.rs` calls use the grid:

- Delete `paint_switch_windows`, `unpaint_switch_windows`,
  `hit_test_switch_windows`, `ThumbCoordinate`, `ThumbnailHitTarget`, and their
  strip-only constants/imports from `painter.rs`.
- Delete the old `capture_window_thumbnail` compatibility wrapper and
  `capture_icon_fallback` from `thumbnail.rs`.
- Keep the app-level `paint`, `unpaint`, `Coordinate`, and `draw_icons` code used
  by the optional Alt+Tab app switcher.

- [ ] **Step 8: Compile and run all unit tests**

Run:

```bash
cargo fmt --all
cargo test --target=x86_64-pc-windows-msvc --all
cargo check --target=x86_64-pc-windows-msvc
```

Expected: all tests and the Windows-target compile PASS.

- [ ] **Step 9: Commit session integration**

```bash
git add src/app.rs src/painter.rs src/thumbnail.rs
git commit -m "feat: integrate cached same-app window grid"
```

---

### Task 7: Add Escape-to-Cancel for Same-App Switching

**Files:**
- Modify: `src/keyboard.rs:1-145`
- Modify: `src/app.rs:30-45`
- Modify: `src/app.rs:245-275`

**Interfaces:**
- Produces: `WM_USER_SWITCH_WINDOWS_CANCEL: u32 = 6022`.
- Consumes: `App::cancel_switch_windows()`.

- [ ] **Step 1: Extract and test keyboard routing decisions**

Introduce:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SwitchKeyAction {
    Advance { id: u32, reverse: bool },
    Cancel { id: u32 },
}

fn switch_key_action(
    id: u32,
    configured_code: u32,
    scan_code: u32,
    shift_pressed: bool,
    switching: bool,
) -> Option<SwitchKeyAction> {
    if scan_code == 0x01 && switching {
        Some(SwitchKeyAction::Cancel { id })
    } else if scan_code == configured_code {
        Some(SwitchKeyAction::Advance {
            id,
            reverse: shift_pressed,
        })
    } else {
        None
    }
}
```

Add:

```rust
#[test]
fn escape_cancels_same_app_switching() {
    assert_eq!(
        switch_key_action(SWITCH_WINDOWS_HOTKEY_ID, 0x29, 0x01, false, true),
        Some(SwitchKeyAction::Cancel {
            id: SWITCH_WINDOWS_HOTKEY_ID
        })
    );
}

#[test]
fn shift_reverses_same_app_switching() {
    assert_eq!(
        switch_key_action(SWITCH_WINDOWS_HOTKEY_ID, 0x29, 0x29, true, false),
        Some(SwitchKeyAction::Advance {
            id: SWITCH_WINDOWS_HOTKEY_ID,
            reverse: true
        })
    );
}

#[test]
fn escape_is_ignored_before_switching_starts() {
    assert_eq!(
        switch_key_action(SWITCH_WINDOWS_HOTKEY_ID, 0x29, 0x01, false, false),
        None
    );
}
```

- [ ] **Step 2: Run the keyboard tests and verify failure**

Run:

```bash
cargo test --target=x86_64-pc-windows-msvc keyboard::tests
```

Expected: compilation fails until `SwitchKeyAction` and `switch_key_action` are
implemented.

- [ ] **Step 3: Route Escape for both switchers**

Add `IS_SWITCHING_WINDOWS`. Pass the matching switcher's active flag into
`switch_key_action`. When the same-app action message is sent, set the flag to
`true`. While its modifier is held and the flag is true, Escape sends
`WM_USER_SWITCH_WINDOWS_CANCEL`, sets `PREVIOUS_KEYCODE` to Escape so modifier
release does not activate, and clears `IS_SWITCHING_WINDOWS`.

Handle the new message in `app.rs`:

```rust
WM_USER_SWITCH_WINDOWS_CANCEL => {
    let app = get_app(hwnd)?;
    app.cancel_switch_windows();
    app.switch_windows_state.modifier_released = true;
}
```

Implement:

```rust
fn cancel_switch_windows(&mut self) {
    if let Some(session) = self.switch_windows_state.overlay.take() {
        self.painter.unpaint_window_grid();
        if unsafe { IsWindow(Some(session.original_hwnd)).as_bool() } {
            set_foreground_window(session.original_hwnd);
        }
    }
}
```

The grid painter does not take focus, but explicitly restoring the original
valid window makes cancellation robust if Windows changes activation while the
overlay is open.

- [ ] **Step 4: Run keyboard and full tests**

Run:

```bash
cargo test --target=x86_64-pc-windows-msvc keyboard::tests
cargo test --target=x86_64-pc-windows-msvc --all
```

Expected: all tests PASS.

- [ ] **Step 5: Commit cancellation support**

```bash
git add src/keyboard.rs src/app.rs
git commit -m "feat: cancel same-app switching with escape"
```

---

### Task 8: Document and Verify the Complete Redesign

**Files:**
- Modify: `src/config.rs:12-145`
- Modify: `src/config.rs:420-470`
- Modify: `README.md:1-85`
- Verify: all modified Rust and INI files

**Interfaces:**
- Produces: the final grid-only `Config` surface and user-facing documentation
  matching the new bundled INI and grid behavior.

- [ ] **Step 1: Remove the obsolete strip configuration surface**

Delete from `src/config.rs`:

- The `TitlePosition` enum.
- `switch_windows_show_title`.
- `switch_windows_title_position`.
- `switch_windows_show_app_icon`.
- Their default initializers and INI parsing branches.
- `test_config_title_position_parse`.

Keep `test_switch_windows_preview_config` from Task 1. Run:

```bash
cargo test --target=x86_64-pc-windows-msvc config::tests
cargo check --target=x86_64-pc-windows-msvc
```

Expected: both commands PASS, proving the completed grid renderer has no
dependency on the removed strip settings.

- [ ] **Step 2: Update the README**

Replace the old same-app feature description with:

```markdown
1. `Alt+Backtick`: switch between windows belonging to the foreground app.

   Hold Alt and press Backtick repeatedly to move through a responsive preview
   grid. Release Alt to activate the selected window, use Shift to move
   backward, or press Escape to cancel.

   The grid shows cached, aspect-correct previews and readable window titles.
   With more than ten windows, selection moves between ten-window pages.
```

Replace the configuration example with the exact contents of
`window-switcher.ini`.

- [ ] **Step 3: Run formatting and static checks**

Run:

```bash
cargo fmt --all --check
cargo clippy --all --all-targets --target=x86_64-pc-windows-msvc
```

Expected: both commands exit 0 with no warnings.

- [ ] **Step 4: Run the complete Windows-target test suite**

Run:

```bash
RUSTFLAGS="--deny warnings" cargo test --target=x86_64-pc-windows-msvc --all
```

Expected: all tests PASS with no warnings.

- [ ] **Step 5: Build the release artifact**

Run:

```bash
cargo build --locked --release --target=x86_64-pc-windows-msvc
```

Expected: build exits 0 and produces
`target/x86_64-pc-windows-msvc/release/window-switcher.exe`.

- [ ] **Step 6: Perform the Windows interaction matrix**

Launch the release binary beside the new `window-switcher.ini` and verify:

| Scenario | Expected result |
| --- | --- |
| 2 windows | One centered row; first forward press selects the previous window |
| 5 windows | One centered row; titles remain readable |
| 6, 7, 9, 10 windows | Balanced two-row layouts of 3+3, 4+3, 5+4, and 5+5 |
| 11+ windows | Ten-card paging follows keyboard selection |
| Portrait/ultrawide windows | Letterbox or pillarbox; never stretch |
| Rapid forward/reverse cycling | No preview recapture or visible flicker |
| Escape | Overlay closes and original window remains foreground |
| Minimized selection | Selected window restores and activates |
| Click card | Window activates immediately |
| Hover card | Hover changes without moving keyboard selection |
| Close/middle-click | Window closes; grid rebalances and stays open |
| External window close | Invalid entry disappears safely |
| Light/dark theme | Surface and text retain contrast |
| Mixed-DPI monitors | Overlay stays within the foreground monitor work area |

- [ ] **Step 7: Commit documentation and verification-ready state**

```bash
git add src/config.rs README.md
git commit -m "docs: describe responsive same-app window grid"
```
