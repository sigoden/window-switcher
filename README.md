# Window Switcher

Window-Switcher offers hotkeys for quickly switching windows on Windows OS:

1. ``` alt+` ```: switch between different windows of the same application.

2. ``` alt+tab ```: switch between open application.


![window-switcher](https://github.com/sigoden/window-switcher/assets/4012553/b3b1b5f0-5433-490b-81b3-c743b81d3236)

## Install

Download from the [Github Release](https://github.com/sigoden/windows-switcher/releases), unzip the `window-switcher.exe` file.  Then, you can simply click on the executable file to run the application directly without the need for installation.

## Configuration

You can configure following items by creating a `window-switcher.ini` file in the same directory as `window-switcher.exe`:

- Hide the tray icon.
- Set custom hotkeys.
- Disable hotkeys for specific apps.
- Turn on/off the switch apps functionality.

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
enable = no

# Hotkey to switch apps
hotkey = alt+tab
```

⚠️ **After changing the configuration, you need to restart.** ⚠️


⚠️ **By default, the functionality to switch between apps (alt+tab) is disabled. To enable it, you must add the following configuration:** ⚠️

```ini
[switch-apps]
enable = yes
```

</details>

## License

Copyright (c) 2023 window-switcher-developers.

window-switcher is made available under the terms of the MIT License, at your option.

See the LICENSE files for license details.