# Window Switcher

Easily switching windows or applications in Windows OS.

1. Use ``` alt+` ``` to switch between different windows of the same application.

2. Use ``` alt+tab ``` to switch between open application.

⚠️ **This feature is turned off by default. You need to add the following configuration to enable.** ⚠️

```ini
[switch-apps]
enable = yes
```

## Install

You can download the [Github Release](https://github.com/sigoden/windows-switcher/releases) from the Github Release page. After downloading, unzip the `window-switcher.exe` file. Then, you can simply click on the executable file to run the application directly without the need for installation.

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