# Window Switcher

Window-Switcher offers hotkeys for quickly switching windows on Windows OS:

1. ```ALT+`(backtick)```: switch between different windows of the same application.

![switch-windows](https://github.com/sigoden/window-switcher/assets/4012553/aca461f2-2381-4477-8ad3-8bb15776675b)

2. ```ALT+TAB```: switch between open application. (disabled by default)

![switch-apps](https://github.com/sigoden/window-switcher/assets/4012553/b3eda588-36eb-45cc-9271-30115de9eb32)


## Install

1. Download `windows-switcher.zip` from the [Github Release](https://github.com/sigoden/windows-switcher/releases).
2. Extract `window-switcher.exe` from zip file. It is a single executable file that does not require installation.
3. Double click on `window-switcher.exe`. Congratulations, you have successfully run Window-Switcher. Try the hotkeys.


## Configuration

You can configure following items by creating a `window-switcher.ini` file in the same directory as `window-switcher.exe`:

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

# 👇👇 Whether to enable switching apps
enable = no 

# Hotkey to switch apps
hotkey = alt+tab

# Ignore minimal windows
ignore_minimal = no
```

## License

Copyright (c) 2023 window-switcher-developers.

window-switcher is made available under the terms of the MIT License, at your option.

See the LICENSE files for license details.