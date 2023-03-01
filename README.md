# Window Switcher

MacOS-like window switcher for Windows OS.

1. Switch between windows of the same app with ``` alt+` ```

![switch-windows](https://user-images.githubusercontent.com/4012553/221805510-ee6a4f2e-e527-4f63-b4a0-080a447d176d.gif)

2. Switch open apps with `alt+tab`

![switch-apps](https://user-images.githubusercontent.com/4012553/221538853-b4793205-23a6-4a27-9f3c-4ff519cd6650.gif)

## Features

- 350k single-file application that can be downloaded from [Github Release](https://github.com/sigoden/windows-switcher/releases), run without installation.
- Support for custom keybindings.
- Support blacklist apps to avoid hotkey override.
- Easily turn on/off `run on startup` using the tray menu.

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

⚠️ **You must manually configure `switch-apps.enable=yes` to enable `alt+tab` to enable open apps.** ⚠️

⚠️ **After changing the configuration, you need to restart for the new configuration to take effect.** ⚠️

## License

Copyright (c) 2023 window-switcher-developers.

window-switcher is made available under the terms of the MIT License, at your option.

See the LICENSE files for license details.