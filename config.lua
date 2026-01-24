nwm.set.master_key(nwm.modifier.Super)
nwm.set.gap(8)
nwm.set.terminal("alacritty")
nwm.set.launcher("dmenu_run")
nwm.set.border_width(2)
nwm.set.border_active_color("#ffdd33")
nwm.set.border_inactive_color("#181818")

nwm.bind("h", nwm.action.focus.left)
nwm.bind("l", nwm.action.focus.right)

nwm.bind("Shift-h", nwm.action.move.left)
nwm.bind("Shift-l", nwm.action.move.right)
nwm.bind("Shift-q", nwm.action.quit)

nwm.bind("Space", nwm.action.launcher)
nwm.bind("Return", nwm.action.terminal)

nwm.bind("w", nwm.action.close)
nwm.bind("2", nwm.action.next_ws)
nwm.bind("1", nwm.action.prev_ws)

nwm.bind("r", nwm.action.reload)

if nwm.first_boot then
    os.execute("pipewire &")
    os.execute("feh --bg-fill  ~/Wallpapers/wall.png &")
end

