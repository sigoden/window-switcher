# AGENTS.md

## What this is

A Windows-only Rust CLI/daemon that provides global hotkeys for switching between windows of the same app (Alt+Backtick) and between apps (Alt+Tab override). Uses Win32 APIs via the `windows` crate. Single binary, no installation.

## Build and test

**Target is always `x86_64-pc-windows-msvc`.** The app uses Win32 APIs and will not compile on Linux or macOS.

```bash
# Build
cargo build --target=x86_64-pc-windows-msvc

# Build release (LTO + size-optimized, used for releases)
cargo build --locked --release --target=x86_64-pc-windows-msvc

# Test (CI runs this with --target)
cargo test --target=x86_64-pc-windows-msvc --all

# Lint
cargo clippy --all --all-targets

# Format check
cargo fmt --all --check
```

CI enforces `RUSTFLAGS=--deny warnings`, so treat warnings as errors.

## Workspace

Two members:
- **root crate (`window-switcher`)** — the main binary and library. Entry: `src/main.rs`, library API: `src/lib.rs`.
- **`tools/inspect-windows`** — a debug utility that depends on the main crate as a library. Useful for inspecting window state during development.

## Architecture

The app is a single-window Win32 message loop:

- `src/main.rs` — entry point, DPI awareness, log setup, single-instance mutex, calls `start()`.
- `src/app.rs` — `App` struct owns the HWND, tray icon, startup manager, switch state. `window_proc` handles all WM_* messages. This is the core of the app.
- `src/config.rs` — INI config loader. Config file lives next to the exe as `window-switcher.ini`. Embedded default config via `include_str!`.
- `src/keyboard.rs` — global hotkey registration via Win32 `RegisterHotKey`.
- `src/foreground.rs` — foreground window change watcher.
- `src/painter.rs` — GDI+ rendering for the app-switcher overlay.
- `src/trayicon.rs` — system tray icon.
- `src/startup.rs` — Windows Task Scheduler integration for auto-start.
- `src/utils/` — Win32 helper wrappers (window enumeration, admin checks, registry, theme detection, etc.).

## Key gotchas

- **`build.rs` embeds a Windows resource file** (`assets/app.rc` + `assets/icon.ico`) via `embed-resource`. This requires the Windows resource compiler (rc.exe), which is only available on Windows or with a cross-compilation toolchain.
- **Config is loaded from the exe's directory**, not CWD. `window-switcher.ini` must be next to the binary.
- **The `#![windows_subsystem = "windows"]`** attribute in `main.rs` suppresses the console window. Don't remove it.
- **Release profile** uses `lto = true`, `strip = true`, `opt-level = "z"` for minimal binary size.
- **Hotkey parsing** uses raw Win32 scan codes (see `config.rs` scan code table), not virtual key codes. If adding new hotkey support, match against the scan code table in `Hotkey::parse`.
- **`SwitchWindowsState` and `SwitchAppsState`** in `app.rs` track cycling state across key taps. Understanding this state machine is critical for modifying switch behavior.
- **`inspect-windows` tool** imports the main crate as a library (`window-switcher = { path = "../.."}`). Changes to public API in the main crate may affect it.

## Testing notes

Tests are unit tests within source files (`#[cfg(test)]`). There are no integration tests or test fixtures. The hotkey parsing tests in `config.rs` are the main test coverage. Since the app is deeply tied to Win32, most behavior is tested manually on Windows.
