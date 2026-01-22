set.master_key(Alt)
set.gap(8)
set.terminal("alacritty")
set.launcher("dmenu_run")
set.border_width(2)
set.border_active_color("#ffdd33")
set.border_inactive_color("#181818")

bind("h", action.focus.left)
bind("l", action.focus.right)

bind("Shift-h", action.move.left)
bind("Shift-l", action.move.right)

bind("Space", action.launcher)
bind("Return", action.terminal)

bind("w", action.close)
bind("2", action.next_ws)
bind("1", action.prev_ws)

bind("r", action.reload)

if first_boot then 
    os.execute("polybar &")
    os.execute("pipewire &")
    os.execute("feh --bg-fill  ~/Wallpapers/wall.png &")
end

