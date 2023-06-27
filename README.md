# Window Switcher

Window-Switcher offers hotkeys for quickly switching windows on Windows OS:

1. ``` alt + ` ```: switch between different windows of the same application.

2. ``` alt + tab ```: switch between open application. (disabled by default)


![window-switcher](https://github.com/sigoden/window-switcher/assets/4012553/0ca54685-32a2-4cf2-8410-d3f8bbacd3ce)

## Install

Download from the [Github Release](https://github.com/sigoden/windows-switcher/releases), unzip the `window-switcher.exe` file.  Then, you can simply click on the executable file to run the application directly without the need for installation.

## Hotkey Trick 

1. Press ``` alt + ` ``` to switch back to the previous working window(The focus window when ``` alt+` ``` was released last time).

2. Keep pressing `alt`, tap ``` ` ``` to switch to the next window in sequence, tap ``` shift + ` ``` to to switch in reverse.

The same rule applies to `alt + tab`.

## Configuration

You can configure following items by creating a `window-switcher.ini` file in the same directory as `window-switcher.exe`:

- Hide the tray icon.
- Set custom hotkeys.
- Disable hotkeys for specific apps.
- Turn on/off the switch apps functionality.
- Controls whether to skip the minimum windows.

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

# Ignore minimal windows
ignore_minimal = no

[switch-apps]

# Whether to enable switching apps
enable = no

# Hotkey to switch apps
hotkey = alt+tab

# Ignore minimal windows
ignore_minimal = no
```

⚠️ **After changing the configuration, you need to restart.** ⚠️

## License

Copyright (c) 2023 window-switcher-developers.

window-switcher is made available under the terms of the MIT License, at your option.

See the LICENSE files for license details.