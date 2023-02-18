# Windows Switcher

Easily switch windows of the same application with a hotkey (``` alt + ` ```) on Windows PC.

- The tiny single-file executable runs without installation.
- Support custom hotkeys.
- Support blacklist apps to avoid hotkey override.

## Get Started

- Downloaded from [Github Release](https://github.com/sigoden/windows-switcher/releases).
- Double-click `Windows-Switcher.exe` to start it. A trayicon will appear.

- Press and hold the `alt` key, tap ``` ` ``` to cycle through windows.
- Press and release ``` alt+` ``` at the same time to switch back to the last focus window.

<details>
<summary>
Experimental switching app
</summary>

- Press and hold the `alt` key, tap `a` to cycle through apps.

- Press and release `alt+a` at the same time to switch back to the last focus app.

</details>
<summary>
</summary>

## Configuration

Create a `windows-switcher.ini` file in the same directory as `windows-switcher.exe`.

The default configuration is as follows:

```ini
# Whether to show trayicon, yes/no
trayicon = yes 

[switch-windows]

# Hotkey to switch windows
hotkey = alt+`

# List of hotkey conflict apps
# e.g. game1.exe,game.exe
blacklist =

[switch-apps]

# Whether to enable switching apps
enable = yes

# Hotkey to switch apps
hotkey = alt+a  # Unable to register system shortcuts such as `alt+tab`
```