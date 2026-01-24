# nwm - another X11 window manager

A basic tiling window manager written in Rust for X11, designed for still undetermined tasks.

## BROKEN STUFF
 - Nothing :3

## Features:
 - Only horizontally window tiling with configurable gaps
 - 10 workspaces
 - Partial EWMH support - support for docks (polybar, ...)
 - Configurable (via lua)

## Installation
Ensure you have Cargo installed then build.
Two binaries will be built (in target/(debug|release)/)- nwm, and nwlog.
Nwm is the window manager, while nwlog is the consumer of logs that are produced by nwm

## Config
Nwm will look for its configuration file in `~/.config/nwm/config.lua` 
Config example is in ./config.lua


### Available configuration settings
 - Gap               : Pixel gap (inner and outer) between windows [default: 0]
 - MasterKey         : Master modifier which gets prepended on all keybinds [default: Super (Mod4)]
 - Terminal          : Default terminal emulator
 - Launcher          : Default launcher application
 - BorderWidth       : Set the border width which indicates focus
 - BorderActiveColor : Set the color of the borders when a window is active
 - BorderInactiveColor : Set the color of the borders when a window is inactive

### Available actions
 - Terminal        : Launch the terminal with the option specified (duh)
 - Launcher        : Launch the launcher specified with the option above
 - CloseWindow     : Close the currently focused window
 - FocusLeft/Right : Focus to the left or right relative to the current focused window
 - MoveLeft/Right  : Move the currently focused window to the left or right
 - Next/PrevWs     : Jump to next/previous workspace
 - ReloadConfig    : Reload the config.lua file

## Startup external programs (for additional services)
Just use os.execute("... &") inside config.lua

## Dependencies:
 - Colored (I'm ashamed that I pull a dependency just for colors)
 - env_logger (Logging to stderr)
 - log (rust pretty much standard logging backend)
 - nix (for nwlog)
 - platform_dirs (duh)
 - x11rb (safe and more ergonomic rust bindings to x11)
 - mlua (really good lua bindings)

