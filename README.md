# Window Switcher

MacOS-like window switcher for Windows OS.

1. Switch between windows of the same app with ``` alt+` ```

![switch-windows](https://user-images.githubusercontent.com/4012553/222900407-e62c4407-414c-40b9-86b1-112d0e227cde.gif)

2. Switch open apps with `alt+tab`

![switch-apps](https://user-images.githubusercontent.com/4012553/221538853-b4793205-23a6-4a27-9f3c-4ff519cd6650.gif)

⚠️ **This feature is turned off by default. You need to add the following configuration to enable.** ⚠️

```ini
[switch-apps]
enable = yes
```

## Install

Download from [Github Release](https://github.com/sigoden/windows-switcher/releases), unzip `window-switcher.exe`, click to run directly without installation.

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

⚠️ **After changing the configuration, you need to restart.** ⚠️

## License

Copyright (c) 2023 window-switcher-developers.

window-switcher is made available under the terms of the MIT License, at your option.

See the LICENSE files for license details.