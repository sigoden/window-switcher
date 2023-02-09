# Windows Switcher

Easily switch windows of the same application with a hotkey (``` alt + ` ```) on Windows PC.

- 250k single file executable
- No installation required, just run it.
- Support custom hotkeys
- Support blacklist apps to avoid hotkey override

## Get Started

- Downloaded from [Github Release](https://github.com/sigoden/windows-switcher/releases).
- Double-click `Windows-Switcher.exe` to start it. A tray icon will appear indicating that the Windows switcher has run successfully.
- Press and hold the `alt` key, tap ``` ` ``` to cycle through windows, press ` alt + ` to switch back to the last focus window.

## Configuration

Create a `windows-switcher.ini` file in the same directory as `windows-switcher.exe`.

The default configuration is as follows:

```ini
# Whether to show trayicon, yes/no
trayicon = yes 

# Hotkey to switch windows
hotkey = alt+`

# List of hotkey conflict apps
# e.g. game1.exe,game.exe
blacklist =
```