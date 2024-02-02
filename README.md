# Window Switcher

Window-Switcher offers hotkeys for quickly switching windows on Windows OS:

1. ```Alt+`(Backtick)```: switch between windows of the same app.

![switch-windows](https://github.com/sigoden/window-switcher/assets/4012553/06d387ce-31fd-450b-adf3-01bfcfc4bce3)

2. ```Alt+Tab```: switch between apps. (disabled by default)

![switch-apps](https://github.com/sigoden/window-switcher/assets/4012553/0c74a7ca-3a48-4458-8d2d-b40dc041f067)

Tips: **Hold `Alt` and click `Backtick/Tab` to cycle through, press `Alt+Backtick/Tab` to switch to the previous one.**

## Install

 Download `windows-switcher.zip` from the [Github Release](https://github.com/sigoden/windows-switcher/releases), extract `window-switcher.exe`, run it. 
> window-switcher.exe is a portable single-file program (less than 500 KB in size). No installation is required.


## Configuration

The window-switcher supports custom shortcuts, enabling and disabling certain functions, and all of these can be set through the configuration file.

The configuration file must be named `window-switcher.ini` and located in the same directory as `window-switcher.exe` for the changes to take effect.  

Here is the default configuration:

```ini
# Whether to show trayicon, yes/no
trayicon = yes 

[switch-windows]

# Hotkey to switch windows
hotkey = alt+`

# List of hotkey conflict apps
# e.g. game1.exe,game2.exe
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

## License

Copyright (c) 2023-2024 window-switcher developers.

window-switcher is made available under the terms of the MIT License, at your option.

See the LICENSE files for license details.