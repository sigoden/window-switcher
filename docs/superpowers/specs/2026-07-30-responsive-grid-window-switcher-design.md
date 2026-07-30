# Responsive Grid Same-App Window Switcher Design

## Summary

Replace the current same-app thumbnail strip with a responsive, keyboard-first
grid inspired by DockDoor's window switcher and adapted to Windows 11 visual
conventions.

The redesign targets the `Alt+Backtick` workflow. It does not add taskbar-hover
previews or change the optional app-level `Alt+Tab` switcher.

The main usability goals are:

- Keep window titles readable with up to ten windows from the same app.
- Make the current selection immediately recognizable.
- Avoid distorted previews, repeated captures, and visible repaint flicker.
- Give the overlay a coherent Windows 11 Fluent visual hierarchy.

## Scope

This work replaces the existing same-app thumbnail strip. It does not retain a
legacy strip mode or preserve the old preview appearance settings.

Included:

- A centered, responsive one- or two-row grid.
- A shared app identity header.
- Readable per-window titles.
- Cached, aspect-fitted window previews.
- Strong selected, hover, and close-button states.
- Keyboard cancellation and reverse cycling.
- Mouse activation and quick-close actions.
- A clean replacement example configuration.

Not included:

- Dock or taskbar hover previews.
- A user-selectable layout system.
- Animations beyond simple state transitions supported by the existing layered
  window renderer.
- Changes to the separate app-level `Alt+Tab` overlay.

## Visual Design

### Overlay

The overlay is centered in the work area of the monitor containing the current
foreground window. Its bounds must remain within that work area at every
supported DPI.

The surface uses:

- A translucent light or dark background selected from the Windows theme.
- Rounded outer corners.
- A soft shadow where supported.
- Consistent DPI-scaled outer padding and inter-card gaps.
- An acrylic-like appearance implemented by the existing layered-window
  renderer; true Acrylic is not required.

### Shared App Header

The header displays:

- The current app icon.
- An app name derived from the executable filename without its extension.
- A compact position indicator such as `3 of 8`.

App identity appears once because every card belongs to the same app. There are
no repeated app-icon badges on individual cards.

### Grid

The number of columns is derived from the window count:

| Window count | Rows | Columns per row |
| --- | --- | --- |
| 2–5 | 1 | The window count |
| 6 | 2 | 3 + 3 |
| 7 | 2 | 4 + 3 |
| 8 | 2 | 4 + 4 |
| 9 | 2 | 5 + 4 |
| 10 | 2 | 5 + 5 |

For a count above ten, the grid is paged in groups of ten. Moving the selection
past a page boundary redraws the next or previous page. The header position
indicator always reports the position in the full list, such as `11 of 14`.
This keeps card size and overlay height stable.

Rows are centered independently so a shorter final row remains visually
balanced. Cards use a consistent size within an overlay session.

### Cards

Each card contains:

1. A preview frame.
2. A single-line, ellipsized window title beneath the preview.
3. A close button shown only while the card is selected or hovered.

Previews preserve the source window's aspect ratio. They are scaled to fit
inside the preview frame and letterboxed with a neutral card color when the
aspect ratios differ. They are never stretched.

The selected card uses the Windows system highlight/accent color for a clearly
visible outline, a brighter card surface, and subtle elevation. Unselected
cards remain legible and are not heavily dimmed. Pointer hover has a distinct
but weaker treatment and does not replace the keyboard selection.

## Interaction Model

- Holding the configured modifier opens the overlay.
- Each press of the configured same-app key advances selection in row-major
  order.
- Adding Shift moves selection backward.
- Releasing the modifier activates the selected window.
- Escape dismisses the overlay without changing the foreground window.
- Clicking a card activates that window immediately.
- Moving the pointer over a card changes only its hover appearance.
- Clicking a visible close button closes that window.
- Middle-clicking a card closes that window when quick actions are enabled.
- After a close action, invalid entries are removed, the grid rebalances, and
  the nearest valid selection remains active.
- Minimized windows are included and restored when activated.

Windows are ordered by most recently used. The foreground window is the first
entry, so an initial forward invocation selects the previously used window.
This preserves the fast two-window toggle behavior.

## Architecture

### `WindowGridLayout`

A pure, platform-data-independent layout component calculates:

- Overlay bounds.
- Header bounds.
- Row and column distribution.
- Card, preview, title, and close-button rectangles.
- Selected-index traversal.
- Pointer hit targets.
- Aspect-fit destination rectangles.

Inputs include the monitor work area, DPI scale, window count, and basic visual
metrics. Keeping layout pure makes the most important behavior unit-testable
without a live Win32 desktop.

### `WindowPreview`

One overlay-session item stores:

- Window handle.
- Window title.
- Cached source dimensions.
- Cached preview bitmap or a capture-failure marker.

The overlay model separately stores the shared app name and icon.

Preview bitmaps are captured once when the overlay opens. Selection changes
reuse those cached bitmaps. Closing or dismissing the overlay releases all
owned GDI objects.

### `WindowGridPainter`

The painter consumes a `WindowGridLayout` and overlay model to draw:

- The translucent themed surface.
- Shared app identity.
- Window position indicator.
- Cards and aspect-fitted previews.
- Titles.
- Selected, hover, and close-button states.

The painter does not enumerate windows, decide ordering, or capture a preview
during an ordinary selection repaint.

### Application State

The existing same-app switching state gains:

- The prepared overlay model.
- Keyboard-selected index.
- Optional pointer-hovered index.
- Optional pointer-hovered close-button index.
- The original foreground window for cancellation.

Window enumeration and MRU ordering remain application responsibilities. The
application prunes handles that become invalid while the overlay is open.

## Capture and Fallback Behavior

Window capture occurs once per item when an overlay session begins. A failed,
empty, or invalid capture produces a neutral preview frame with the shared app
icon centered inside it.

Other fallbacks:

- An empty title renders as `Untitled`.
- A missing app icon leaves the header icon slot empty without shifting other
  content incorrectly.
- An unavailable executable name renders as `Application`.
- A window that closes externally is removed before activation or repaint.
- If no selectable windows remain, the overlay closes.

Capture failures must not abort the switcher or prevent other windows from
appearing.

## Configuration

There is no backward-compatibility requirement for the current preview styling
keys. The bundled `window-switcher.ini` is replaced with this schema:

```ini
trayicon = yes

[switch-windows]
hotkey = alt+`
blacklist =
ignore_minimal = no
thumbnail_preview = yes
enable_quick_actions = yes
only_current_desktop = auto

[switch-apps]
enable = no
hotkey = alt+tab
ignore_minimal = no
override_icons =
only_current_desktop = auto

[log]
level = info
path =
```

Window titles and the shared app header are required parts of the grid and are
not individually configurable. Remove `show_title`, `title_position`, and
`show_app_icon` from `Config` and its parser.

## Testing

### Automated Tests

Add unit tests for:

- Row balancing and centered rows for counts two through ten.
- Page boundaries and page transitions for counts above ten.
- Monitor-bound layout at 100%, 125%, 150%, and 200% scale.
- Row-major forward and reverse traversal.
- Card and close-button hit testing.
- Hover and keyboard-selection state independence.
- Aspect-fit rectangles for portrait, landscape, square, and ultrawide sources.
- Rebalancing and selection clamping after a close.
- Empty-title fallback.
- The replacement configuration schema and defaults.

### Windows Verification

Manually verify on Windows:

- Light and dark themes.
- 100% through 200% display scaling.
- Multiple monitors with different scaling.
- Two, five, six, seven, nine, and ten same-app windows.
- Minimized windows.
- Capture failure and icon fallback.
- Rapid forward and reverse cycling.
- Release-to-activate and Escape-to-cancel.
- Mouse activation, hover, close button, and middle-click close.
- Windows closing externally while the overlay is visible.

Run the repository checks required by `AGENTS.md`:

```bash
cargo test --target=x86_64-pc-windows-msvc --all
cargo clippy --all --all-targets
cargo fmt --all --check
```

Treat warnings as errors.

## Acceptance Criteria

The redesign is complete when:

- Titles remain readable with ten same-app windows on a 1920×1080 display at
  100% scaling.
- Every preview preserves its source aspect ratio.
- Rapid cycling reuses cached previews and does not visibly recapture or
  flicker.
- The selected card is immediately distinguishable in both light and dark
  themes.
- The overlay remains inside the active monitor work area at supported DPI
  scales.
- Escape reliably cancels without activating another window.
- Close actions safely reflow the grid and preserve a valid selection.
- All automated repository checks pass on the Windows MSVC target.
