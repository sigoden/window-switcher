# Window Switcher

Window-Switcher offers hotkeys for quickly switching windows on Windows OS:

1. ```Alt+`(Backtick)```: switch between windows of the same app.

![switch-windows](https://github.com/sigoden/window-switcher/assets/4012553/06d387ce-31fd-450b-adf3-01bfcfc4bce3)

2. ```Alt+Tab```: switch between apps. (disabled by default)

![switch-apps](https://github.com/sigoden/window-switcher/assets/4012553/0c74a7ca-3a48-4458-8d2d-b40dc041f067)

Tips: **Hold `Alt` and strike `Backtick/Tab` to cycle through, press `Alt + Backtick/Tab` to switch to the previous one.**

## Install

 Download `windows-switcher.zip` from the [Github Release](https://github.com/sigoden/windows-switcher/releases), extract `window-switcher.exe`, run it. 

> window-switcher.exe is a portable single-file program (less than 1 MB in size). No installation is required.


## Configuration

Window-Switcher offers various customization options to tailor its behavior to your preferences. You can define custom keyboard shortcuts, enable or disable specific features, and fine-tune settings through a configuration file.

To personalize Window-Switcher, you'll need a configuration file named `window-switcher.ini`. This file should be placed in the same directory as the `window-switcher.exe` file. Once you've made changes to the configuration, make sure to restart Window-Switcher so your new settings can take effect.

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

## Running as Administrator (Optional)

While not mandatory, running `window-switcher.exe` with administrator privileges unlocks its full potential, especially when working with system applications like Task Manager that require elevated permissions. This allows for smoother interactions with all types of applications.

**Administrator Privileges and Startup Option:**

* **Running as Admin + Enabling Startup:** Launches `window-switcher.exe` with administrator privileges every time you start your computer.
* **Running without Admin + Enabling Startup:** Launches `window-switcher.exe` with regular user privileges at startup.

## License

Copyright (c) 2023-2024 window-switcher developers.

window-switcher is made available under the terms of the MIT License, at your option.

See the LICENSE files for license details.