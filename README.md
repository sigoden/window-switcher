# Window Switcher

MacOS-like window switcher for Windows OS.

1. Switch between windows of the same app with ``` alt+` ```

![switch-windows](https://user-images.githubusercontent.com/4012553/221805510-ee6a4f2e-e527-4f63-b4a0-080a447d176d.gif)

2. Switch between application windows with `alt+tab`

![switch-apps](https://user-images.githubusercontent.com/4012553/221538853-b4793205-23a6-4a27-9f3c-4ff519cd6650.gif)

## Get Started

- Download and unzip window-switcher from [Github Release](https://github.com/sigoden/window-switcher/releases).
- Double-click `window-switcher.exe` to start it. A trayicon will appear.
- Done. Try use hotkey to switch windows.

## Configuration

You can configure window-switcher by creating a `window-switcher.ini` file in the same directory as `window-switcher.exe`.

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

**You have to manually configure `switch-apps.enable=yes` to replace the system builtin `alt+tab` switcher**;

## License

Copyright (c) 2023 window-switcher-developers.

window-switcher is made available under the terms of the MIT License, at your option.

See the LICENSE files for license details.