# Windows Switcher

MacOS-like window switcher for Windows OS.

1. Switch between windows of the same app with ``` alt+` ```

![snapshot2](https://user-images.githubusercontent.com/4012553/221538973-89468c27-c0e2-45ca-bfae-ac45bd5ad6df.gif)

2. Switch between application windows with `alt+a`

![snapshot1](https://user-images.githubusercontent.com/4012553/221538853-b4793205-23a6-4a27-9f3c-4ff519cd6650.gif)

## Get Started

- Download and unzip windows-switcher from [Github Release](https://github.com/sigoden/windows-switcher/releases).
- Double-click `Windows-Switcher.exe` to start it. A trayicon will appear.
- Done. Try use hotkey to switch windows.

## Configuration

If you need to custom hotkeys, you can create a `windows-switcher.ini` file in the same directory as `windows-switcher.exe`, then add custom configuration.

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
hotkey = alt+a # Unable to register system hotkey such as alt+tab, win+tab
```

## License

Copyright (c) 2022 windows-switcher-developers.

windows-switcher is made available under the terms of the MIT License, at your option.

See the LICENSE files for license details.