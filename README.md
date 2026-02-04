# nwm - another X11 window manager
A basic tiling window manager written in Rust for X11, designed for still undetermined tasks.

## Intro
NWM (not xsoders [nwm](https://github.com/xsoder/nwm)) is a [tiling window manager](https://en.wikipedia.org/wiki/Tiling_window_manager)
for [x11](https://en.wikipedia.org/wiki/X_Window_System), written in Rust (btw).

## Features:
 - Multiple layout modes: Horizontal, vertical, master
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

## Configuration guide
All configuration is done in lua. So far all standard lua modules are enabled, 
because at the end of the day this is YOUR config, that either you write or copy it from someone else, therefore for convenience you can abuse
standard lua :)


### Master key
To start with anything you do in nwm is prefixed by your **master key** (refered later to by **MK**), this is explained later.
The available keys are in the `nwm.modifier` table and they are:
 - Super
 - Alt
 - Control
You can set **MK** by calling the function `nwm.set.master_key(<key>)`.

### Actions
Actions can: 
 - Move windows around the workspace and move to other ones
 - Shift focus 
 - Reload the config
 - Switch between available layouts
 - Change the gap size
 - Change the master layout ratio
 - Launch root programs

Actions can be bound with the `nwm.bind` function, which takes
a string with the bind, and the action in the `nwm.action` table.
A bind is arbitrarily long sequence of modifiers and final non-modifier key.
Each element is separated by hyphens (-).
Some example valid keybinds include:
 - "1"
 - "2"
 - "p"
 - "Space"
 - "Return"
 - "Shift-2"
 - "Control-a"
 - "Control-Shift-2"

### Shifting window focus
To move your focus around the actions within the `nwm.action.focus` table are provided.
For example to bind "MasterKey-h" to be the keybind to focus to the left window you do this:
```lua
    nwm.bind("h", nwm.action.focus.left)
```

### Moving windows within the workspace
To move a window around the actions within the `nwm.action.move` table are provided.
For example to bind "MasterKey-Shift-h" to be the keybind to move to the left window you do this:
```lua
    nwm.bind("Shift-h", nwm.action.move.left)
```

### Closing windows
To close a window the `nwm.action.close` action is provided

### Launching "Root" applications
The so called "root" applications are your terminal and app launcher.
Each can be bound with the same `nwm.bind` function and the action for each is `nwm.action.launcher` and `nwm.action.terminal`.
Example:
```lua
    nwm.bind("Space", nwm.action.launcher)
    nwm.bind("Return", nwm.action.terminal)
```

### Workspaces
To show one of the 10 (0-9) workspaces the action `nwm.action.ws(0-9)` is given.
Whereas to move a window to a specific workspace the action `nwm.action.move_to_ws(0-9)` is provided.

### Cycling between layouts
To switch between the three available layouts (horizontal, vertical, master stack) actions `nwm.action.next_layout` and `nwm.action.prev_layout` are provided.

### Changing gaps/master ratio at runtime
To change the gap size actions `nwm.action.gap_up` and `nwm.action.gap_down` are given.
To change the master split ratio actions `nwm.action.master_ratio_up` and `nwm.action.master_ratio_down` are given.

### Reloading the config
The action `nwm.action.reload` is given to be used with the `nwm.bind` function


### Launching programs on startup
To launch for example pipewire or feh and not to launch them multiple times after each reload
The boolean `nwm.first_boot` value is given

### Variables
The configuration variables are set by the functions within the `nwm.set` table.
Brief explanation of each one:
 - `nwm.set.master_key()`                - Explained in the master key section
 - `nwm.set.gap()`                       - Sets the gap between each window (inner and outer), takes a positive integer
 - `nwm.set.master_ratio()`              - Sets the master ratio (explained later), takes a positive floating point number (0..1)
 - `nwm.set.terminal()`                  - Sets the command to be run when `nwm.action.terminal` is triggered, takes a string
 - `nwm.set.launcher()`                  - Sets the command to be run when `nwm.action.launcher` is triggered, takes a string
 - `nwm.set.border_width()`              - Sets the border width around each window, takes a positive integer
 - `nwm.set.border_active_color()`       - Sets the border color for the focused window, takes a string in this format: "#RRGGBB"
 - `nwm.set.border_inactive_color()`     - Sets the border color for the not focused windows, takes a string in this format: "#RRGGBB"

## Available layout kinds
 - The one NWM had from it's start, its the horizontal layout, nothing to write home about it just splits windows vertically.
(Graphic design is my passion)
```text
┌──────┬──────┬──────┐
│(1)   │(2)   │(3)   │
│      │      │      │
│      │      │      │
│      │      │      │
└──────┴──────┴──────┘ 
```
- Another one is the vertical layout, splits the windows horizontally
(Again, graphic design is my passion)
```text
┌───────────────────────────────┐
│(1)                            │
│                               │
├───────────────────────────────┤
│(2)                            │
│                               │
├───────────────────────────────┤
│(3)                            │
│                               │
└───────────────────────────────┘ 
```
- And the last one (so far) is probably the most useful of them all is the master stack layout.
Basically it keeps one **master** window which takes up **master_ratio**% (50% by default) of the width of the screen.
The remainder of the screen gets given to the children windows, spliting horizontally
(Once again, my graphic design "skills" prevail)
```text
┌───────────────┬───────────────┐
│(1)            │(2)            │
│               │               │
│               ├───────────────┤
│               │(3)            │
│               │               │
│               ├───────────────┤
│               │(4)            │
│               │               │
└───────────────┴───────────────┘ 
```

### Runtime information
NWM exposes runtime information through the `nwm.info` table. 
This table is automatically updated as the window manager state changes.
Available Info Fields:

`nwm.info.version` - NWM version string
`nwm.info.name` - Package name ("nwm")
`nwm.info.hostname` - System hostname
`nwm.info.user` - Current user
`nwm.info.display` - X11 display (e.g., ":0")
`nwm.info.workspace_count` - Total number of workspaces (10)
`nwm.info.current_workspace` - Currently active workspace (0-9)
`nwm.info.focused_window` - X11 window ID of focused window
`nwm.info.window_count` - Number of windows on current workspace
`nwm.info.gap` - Current gap size
`nwm.info.master_ratio` - Current master layout ratio
`nwm.info.screen_width` - Screen width in pixels
`nwm.info.screen_height` - Screen height in pixels

#### Example usage
```lua
    -- Print workspace info when switching
    nwm.bind("d", function()
        print("Workspace " .. nwm.info.current_workspace .. " has " .. nwm.info.window_count .. " windows")
    end)
```

### Lua callbacks
You can bind Lua functions directly to keybindings 
instead of only using the built-in `nwm.action` constants.

#### Syntax
```lua
    nwm.bind("keybind", function()
        -- Your Lua code here
    end)
```

#### Examples
 - Print runtime info:
```lua
    nwm.bind("d", function()
        print("Current workspace: " .. nwm.info.current_workspace)
        print("Windows: " .. nwm.info.window_count)
    end)
```
 - Send notifications
 ```lua
    nwm.bind("n", function()
        local ws = nwm.info.current_workspace
        local count = nwm.info.window_count
        os.execute(string.format('notify-send "NWM" "Workspace %d: %d windows"', ws, count))
    end)
 ```
 - Access ENV variables
 ```lua
     nwm.bind("v", function()
        local editor = os.getenv("EDITOR") or "vim" -- Or emacs if you wish :)
        os.execute(editor .. " ~/notes.txt &")
    end)
 ```

#### Advanced examples
 - Workspace-Specific Commands:
```lua
    nwm.bind("t", function()
        local ws = nwm.info.current_workspace
        if ws == 0 then
            os.execute("firefox &")
        elseif ws == 1 then
            os.execute("spotify &")
        else
            os.execute("alacritty &")
        end
    end)
```

#### Hooks
You can also register callbacks to be run on different NWM events.
Three are available so far:
 - When a window is added `nwm.hook.add_window`
 - When a window is removed `nwm.hook.remove_window`
One of these events should be used in the `nwm.on` function to register them.
Multiple can be registered per event.
##### Example:
 - TODO: No reasonable example exists for now

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
