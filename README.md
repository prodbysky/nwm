# nwm - another X11 window manager
A basic tiling window manager written in Rust for X11, designed for still undetermined tasks.

## Intro
NWM (not xsoders [nwm](https://github.com/xsoder/nwm)) is a [tiling window manager](https://en.wikipedia.org/wiki/Tiling_window_manager)
for [x11](https://en.wikipedia.org/wiki/X_Window_System), written in Rust (btw).

## Features:
 - Only horizontally window tiling with configurable gaps
 - 10 workspaces
 - Partial EWMH support - support for docks (polybar, ...)
 - Configurable (via lua)
 - Floating window support :)
 - Fullscreen support

## Installation
Ensure you have Cargo installed then build. As this is for X11 be on x11 not wayland.
I've tested building this repo within a clean install of [alpine linux](https://www.alpinelinux.org/),
the things needed to build this from source were git and cargo.
Two binaries will be built (in target/(debug|release)/)- nwm, and nwlog.
Nwm is the window manager, while nwlog is the consumer of logs that are produced by nwm

## Using nwm
On first run if you haven't taken the example config from `config.lua` in the root of 
this repository, nwm will start in a "crippled" mode, as it failed to load the config in 
`~/.config/nwm/config.lua`, it will have some keybinds but it is recommended that
you provide your own configuration. The before mentioned config includes all configuration
"scenarios that you will encounter, when configuring.
On startup you will see a blank screen (that you will probably want to fill up with [feh](https://feh.finalrewind.org/) for example).
As mentioned bar/strut support is implemented so you are free to use polybar for more status info.
If you are facing issues with nwm, check the logs in `/tmp/nwm.log` or run `nwlog`. 

## Contribution / issue reporting
If you want to contribute, fork the repository, do not make changes on the master branch
always make a separate branch, have clear commit messages, in the PR include your reasons what does this accomplish.
Make sure if you use any unsafe code note that in the description.
Make sure you test the change via the `test.sh` script, which uses Xephyr to open a new X display (:1) and runs nwm within.
Cargo fmt is mandatory (yes I don't like the way it formats but it keeps the formatting consistent).

## Dependencies:
 - Colored (I'm ashamed that I pull a dependency just for colors)
 - env_logger (Logging to stderr)
 - log (rust pretty much standard logging backend)
 - nix (for nwlog)
 - x11rb (safe and more ergonomic rust bindings to x11)
 - mlua (really good lua bindings)

 ## Thanks
 Thanks for checking this hobby project of mine, and especially thanks for any help given as I'm only an unemployed guy with no
 experience in the real world. Any contributions (that are useful of course) mean the world to me.
