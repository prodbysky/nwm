# nwm - another X11 window manager

A basic tiling window manager written in Rust for X11, designed for still undetermined tasks.

## Features:
 - Only horizontally window tiling with configurable gaps
 - 10 workspaces
 - Partial EWMH support - support for docks (polybar, ...)
 - Configurable keybinds

## Installation
Ensure you have Cargo installed then build.
Two binaries will be built (in target/(debug|release)/)- nwm, and nwlog.
Nwm is the window manager, while nwlog is the consumer of logs that are produced by nwm

## Config
Nwm will look for its configuration file in `~/.config/nwm/config.nwc`, if it does not exist 
it will be written with the default configuration
set gap 10
set master_key Super
set terminal "alacritty"
set launcher "rofi -show drun"
 - Config example: 
```
Set MasterKey Alt
Set Gap 8
Set Terminal alacritty
Set Launcher dmenu_run

Do FocusLeft on h
Do FocusRight on l

Do MoveLeft on Shift-h
Do MoveRight on Shift-l


Do CloseWindow on w

Do NextWs on 2
Do PrevWs on 1

Do Launcher on Space
Do Terminal on Return
```
### Available configuration settings
 - Gap       : Pixel gap (inner and outer) between windows [default: 0]
 - MasterKey : Master modifier which gets prepended on all keybinds [default: Super (Mod4)]
 - Terminal  : Default terminal emulator
 - Launcher  : Default launcher application

### Available actions
 - Terminal        : Launch the terminal with the option specified (duh)
 - Launcher        : Launch the launcher specified with the option above
 - CloseWindow     : Close the currently focused window
 - FocusLeft/Right : Focus to the left or right relative to the current focused window
 - MoveLeft/Right  : Move the currently focused window to the left or right
 - Next/PrevWs    : Jump to next/previous workspace

## Startup script (for additional services)
Nwm will run a shell script (`~/.config/nwm/run.sh`) on startup where you can just write shell.
Make sure your run.sh does not block.

## Dependencies:
 - Colored (I'm ashamed that I pull a dependency just for colors)
 - env_logger (Logging to stderr)
 - log (rust pretty much standard logging backend)
 - nix (for nwlog)
 - platform_dirs (duh)
 - x11rb (safe and more ergonomic rust bindings to x11)

