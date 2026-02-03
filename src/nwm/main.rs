mod better_x11rb;
mod ewmh;
mod layout;
mod lua_cfg;
mod multi_log;
mod nw_log_connection;
mod workspace;

use std::{collections::HashMap, process::Command};

use better_x11rb::WindowId;

use log::{error, info, warn};

struct Nwm {
    x11: better_x11rb::X11RB,
    workspaces: [workspace::Workspace; 10],
    curr_workspace: usize,
    last_focused: Option<WindowId>,
    running: bool,
    /// Cursor X
    last_x: i16,
    /// Cursor Y
    last_y: i16,
    ewmh: ewmh::Ewmh,
    struts: HashMap<WindowId, ewmh::Strut>,
    gap: u8,
    master_ratio: f32,
    binds: Vec<Bind>,
    terminal: String,
    launcher: String,
    border_width: u8,
    active_border_color: u32,
    inactive_border_color: u32,
    suppress_cursor_focus: bool,
    layout_man: layout::LayoutManager,
    lua: Option<mlua::Lua>,
}

impl Nwm {
    /// Updates runtime info in the Lua context
    fn update_lua_runtime_info(&self) {
        if let Some(lua) = &self.lua {
            if let Ok(nwm_table) = lua.globals().get::<mlua::Table>("nwm") {
                if let Ok(info_table) = nwm_table.get::<mlua::Table>("info") {
                    let _ = info_table.set("current_workspace", self.curr_workspace);
                    let _ = info_table.set("focused_window", self.focused().unwrap_or(0));
                    let _ = info_table.set("window_count", self.curr_ws().window_count());
                    let _ = info_table.set("gap", self.gap as usize);
                    let _ = info_table.set("master_ratio", self.master_ratio);
                    
                    let (w, h) = self.x11.screen_size();
                    let _ = info_table.set("screen_width", w);
                    let _ = info_table.set("screen_height", h);
                }
            }
        }
    }

    /// "Applies" the lua config in `~/.config/nwm/config.lua`, by returning some values that
    /// probably need to be factored out to a struct, which I'm not doing right now since I'm just
    /// writing docs
    fn apply_lua_config(
        conf: lua_cfg::Config,
        x11: &mut better_x11rb::X11RB,
    ) -> (u8, Vec<Bind>, String, String, u32, u32, u8, f32, Option<mlua::Lua>) {
        let settings = conf.settings;

        let mut binds = Vec::new();

        for b in conf.binds {
            let mask = b
                .combo
                .prefixes
                .iter()
                .map(|k| match k {
                    lua_cfg::SpecialKey::Alt => ModMask::M1,
                    lua_cfg::SpecialKey::Shift => ModMask::SHIFT,
                    lua_cfg::SpecialKey::Control => ModMask::CONTROL,
                    lua_cfg::SpecialKey::Super => ModMask::M4,
                })
                .fold(ModMask::default(), |acc, it| acc | it);

            x11.grab_key(mask, b.combo.key.into_x11rb());

            binds.push(Bind {
                action: b.action,
                bind: b.combo,
            });
        }

        (
            settings.gap as u8,
            binds,
            settings.terminal,
            settings.launcher,
            settings.border_active_color,
            settings.border_inactive_color,
            settings.border_width as u8,
            settings.master_ratio,
            conf.lua,
        )
    }

    /// If a focused window for the current workspace exists moves it to the specified workspace
    /// number
    fn move_focused_to_ws(&mut self, ws: usize) {
        if let Some(id) = self.curr_ws().get_focused_id() {
            if self.curr_ws_mut().is_floating(id) {
                if let Some(g) = self.curr_ws().get_geometry(id) {
                    self.workspaces[ws].push_float_window(id, *g);
                }
            } else {
                self.workspaces[ws].push_window(id);
            }
            self.x11.unmap_window(id);
            self.curr_ws_mut().remove_window(id);
        }
        self.update_lua_runtime_info();
    }

    /// Reloads the lua config with `nwm.first_boot` being false
    /// so that the users startup programs don't run multiple times
    /// if the user is experimenting with their config
    fn reload_config(&mut self) {
        let conf = match lua_cfg::load_config(true) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to reload lua config: {e:?}");
                return;
            }
        };

        self.binds.clear();

        let (gap, binds, terminal, launcher, active, inactive, width, master_ratio, lua) =
            Self::apply_lua_config(conf, &mut self.x11);

        self.gap = gap;
        self.binds = binds;
        self.terminal = terminal;
        self.launcher = launcher;
        self.active_border_color = active;
        self.inactive_border_color = inactive;
        self.border_width = width;
        self.master_ratio = master_ratio;
        self.lua = lua;

        for ws in self.workspaces.clone() {
            for w in ws.windows() {
                self.set_window_border_width(*w, self.border_width);
            }
            for w in ws.floating_window_ids() {
                self.set_window_border_width(*w, self.border_width);
            }
        }

        self.update_lua_runtime_info();
        info!("Reloaded lua config");
    }

    /// Initializes nwm, ewmh controller, loads the lua config, inits loggers
    pub fn create(display_name: &str) -> Option<Self> {
        let file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open("/tmp/nwm.log")
            .unwrap();

        multi_log::MultiLog::init(
            vec![
                Box::new(env_logger::Logger::from_default_env()),
                Box::new(nw_log_connection::NwLogLog::init(file)),
            ],
            log::Level::Trace,
        );
        let mut x11_ab = better_x11rb::X11RB::init()?;

        info!("Succesfully initialized display {} ", display_name);

        let conf = lua_cfg::load_config(false).unwrap_or_else(|_| {
            warn!("Failed to load config on startup using barebones default config");
            lua_cfg::Config::default()
        });
        let (gap, binds, terminal, launcher, active, inactive, width, master_ratio, lua) =
            Self::apply_lua_config(conf, &mut x11_ab);

        info!("Everything went well in initialization :DD");
        if launcher.is_empty() {
            warn!("Launcher wasn't set to a program");
        }
        if terminal.is_empty() {
            warn!("Terminal wasn't set to a program");
        }

        let mut ewmh = ewmh::Ewmh::new(&mut x11_ab);
        ewmh.switch_active_desktop(&mut x11_ab, 0);

        Some(Self {
            x11: x11_ab,
            workspaces: Default::default(),
            curr_workspace: 0,
            gap,
            running: true,
            last_x: 0,
            last_y: 0,
            binds,
            launcher,
            terminal,
            struts: HashMap::new(),
            last_focused: None,
            active_border_color: active,
            inactive_border_color: inactive,
            border_width: width,
            suppress_cursor_focus: false,
            layout_man: layout::LayoutManager::default(),
            master_ratio,
            ewmh,
            lua,
        })
    }

    /// Focuses to the specified window `id` and warps the cursor to its center
    fn refocus_and_warp(&mut self, id: WindowId) {
        if let Some((_, r)) = self
            .tiled_window_rects()
            .into_iter()
            .find(|(w, _)| *w == id)
        {
            let cx = r.x + r.w / 2;
            let cy = r.y + r.h / 2;
            self.x11
                .conn
                .warp_pointer(
                    x11rb::NONE,
                    self.x11.root_window(),
                    r.x,
                    r.y,
                    r.w as u16,
                    r.h as u16,
                    cx,
                    cy,
                )
                .unwrap();
        }

        self.set_focus(id);
    }

    /// Goes through all struts that were collected and returns the max of their requested size
    /// Used for layouting
    fn get_reserved_space(&self) -> Reserve {
        let mut p = Reserve::default();

        for s in self.struts.values() {
            p.y0 = p.y0.max(s.top);
            p.y1 = p.y1.max(s.bottom);
            p.x0 = p.x0.max(s.left);
            p.x1 = p.x1.max(s.right);
        }

        p
    }

    /// Switches to the workspace (current workspace - 1)
    /// NOTE: Does not wrap around
    fn focus_next_ws(&mut self) {
        self.switch_ws((self.curr_workspace + 1).clamp(0, 9));
    }

    /// Switches to the workspace (current workspace - 1)
    /// NOTE: Does not wrap around
    fn focus_prev_ws(&mut self) {
        if self.curr_workspace == 0 {
            return;
        }
        self.switch_ws((self.curr_workspace - 1).clamp(0, 9));
    }

    /// Returns the focused window in the current workspace
    fn focused(&self) -> Option<WindowId> {
        self.workspaces[self.curr_workspace].get_focused_id()
    }

    /// If a focused window exists closes it
    fn close_focused(&mut self) {
        if let Some(w) = self.focused() {
            self.close_window(w);
        }
    }

    /// Sends a request to x11 to close a window identified by `id`
    fn close_window(&mut self, id: WindowId) {
        self.x11.close_window(id);
        // self.curr_ws_mut().remove_window(id);
    }

    /// Returns a exclusive reference to the currently active workspace
    fn curr_ws_mut(&mut self) -> &mut workspace::Workspace {
        &mut self.workspaces[self.curr_workspace]
    }

    /// Returns a shared reference to the currently active workspace
    fn curr_ws(&self) -> &workspace::Workspace {
        &self.workspaces[self.curr_workspace]
    }

    /// Unmaps all windows that are not the one that requested to be fullscreened
    /// after that moves the fullscreened window to 0x0 and makes it take up the whole screen
    fn set_fullscreen(&mut self, id: WindowId) {
        self.curr_ws_mut().set_fullscreen_id(Some(id));
        for w in self.curr_ws().windows().iter().filter(|w_id| **w_id != id) {
            _ = self.x11.conn.unmap_window(*w);
        }
        let (sw, sh) = self.x11.screen_size();
        self.x11.move_window(id, 0, 0);
        self.x11.resize_window(id, sw as u32, sh as u32);
    }

    /// Maps all windows that were (hopefully) previously hidden by fullscreening a window
    /// and maps them, after that the layout method is called, and finally the fullscreened ID is
    /// cleared
    fn unset_fullscreen(&mut self) {
        if let Some(id) = self.curr_ws().get_fullscreen_id() {
            for w in self.curr_ws().windows().iter().filter(|w_id| **w_id != id) {
                _ = self.x11.conn.map_window(*w);
            }
        }
        self.layout();
        self.curr_ws_mut().set_fullscreen_id(None);
    }

    /// Starts up the window manager :)
    pub fn run(mut self) {
        info!("Keybindings were setup");
        self.update_lua_runtime_info();

        while self.running {
            let event = match self.x11.next_event() {
                Ok(e) => e,
                Err(x11rb::errors::ConnectionError::IoError(e)) => {
                    error!("Failed to get X11 event due to an IO error, aborting :( : {e}");
                    self.running = false;
                    return;
                }
                Err(e) => {
                    error!("Failed to get X11 event due to: {e}");
                    continue;
                }
            };

            match event {
                Event::MapRequest(e) => self.add_window(e),
                Event::UnmapNotify(e) => self.remove_window(e),
                Event::KeyPress(e) => {
                    for b in &self.binds.clone() {
                        b.try_do(&mut self, e);
                    }
                }
                Event::MotionNotify(_) => {
                    if self.suppress_cursor_focus {
                        continue;
                    }
                    let (x, y) = self.x11.mouse_pos();
                    if self.last_x != x || self.last_y != y {
                        let rects = self.tiled_window_rects();
                        for (i, r) in rects.iter() {
                            if x > r.x && x < r.x + r.w {
                                self.curr_ws_mut().set_focused_id(*i);
                                self.set_focus(*i);
                            }
                        }
                        self.last_x = x;
                        self.last_y = y;
                    }
                }
                Event::EnterNotify(e) => {
                    self.curr_ws_mut().set_focused_id(e.event);
                    self.set_focus(e.event);
                }
                Event::KeyRelease(_) => {}
                Event::MappingNotify(_) => {}
                Event::ConfigureRequest(_) => self.layout(),
                Event::ClientMessage(e) => {
                    if let Some(state) = self.ewmh.get_fullscreen_msg(e) {
                        match state {
                            ewmh::FullscreenMessage::EnableFullscreen => {
                                self.set_fullscreen(e.window);
                            }
                            ewmh::FullscreenMessage::DisableFullscreen => {
                                self.unset_fullscreen();
                            }
                            ewmh::FullscreenMessage::ToggleFullscreen => {}
                        }
                    }
                }
                Event::PropertyNotify(e) => {
                    if let Some(strut) = self.ewmh.get_strut(&mut self.x11, e.window) {
                        self.struts.insert(e.window, strut);
                        self.layout();
                    }
                }
                Event::DestroyNotify(e) => {
                    self.struts.remove(&e.window);
                    self.layout();
                }

                Event::CreateNotify(_) | Event::MapNotify(_) | Event::ConfigureNotify(_) => {}
                _ => {
                    warn!("Skipping event: {:#?}", event);
                }
            }
        }
    }

    /// Finds which window does the cursor overlaps, by first checking the floating windows and then
    /// if not a single floating window overlaps with the cursor checks the tiled ones
    fn focus_on_pointer(&mut self) {
        let rects = self.floating_window_rects();

        for (id, r) in rects.iter() {
            if self.last_x > r.x
                && self.last_x < r.x + r.w
                && self.last_y > r.y
                && self.last_y < r.y + r.h
            {
                self.set_window_border_pixel(*id, self.inactive_border_color);
                self.curr_ws_mut().set_focused_id(*id);
                self.set_focus(*id);
                return;
            }
        }

        let rects = self.tiled_window_rects();
        for (i, r) in rects.iter() {
            if self.last_x > r.x && self.last_x < r.x + r.w {
                self.set_window_border_pixel(*i, self.inactive_border_color);
                self.curr_ws_mut().set_focused_id(*i);
                self.set_focus(*i);
                return;
            }
        }
    }

    fn set_window_border_pixel(&mut self, w: WindowId, color: u32) {
        _ = self
            .x11
            .conn
            .change_window_attributes(w, &ChangeWindowAttributesAux::new().border_pixel(color));
    }

    fn set_window_border_width(&mut self, w: WindowId, width: u8) {
        _ = self
            .x11
            .conn
            .configure_window(w, &ConfigureWindowAux::new().border_width(width as u32))
            .map_err(|e| {
                warn!("Failed to set windows {w} border width to {width}: {e}");
            });
    }

    /// Switches workspaces by first unmapping (hiding) windows from the current workspace
    /// and maps (pops up) all windows from the new workspace
    fn switch_ws(&mut self, new_ws: usize) {
        if new_ws >= self.workspaces.len() || new_ws == self.curr_workspace {
            return;
        }

        let old_ws = self.curr_workspace;

        for &w in self.workspaces[old_ws].windows() {
            self.x11.unmap_window(w);
        }

        for w in self.workspaces[old_ws].floating_window_ids() {
            self.x11.unmap_window(*w);
        }

        self.curr_workspace = new_ws;

        for &w in self.workspaces[new_ws].windows() {
            self.x11.map_window(w);
        }

        for w in self.workspaces[new_ws].floating_window_ids() {
            self.x11.map_window(*w);
        }

        self.ewmh.switch_active_desktop(&mut self.x11, new_ws);

        self.layout();
        self.focus_on_pointer();
        self.update_lua_runtime_info();
    }

    /// Returns all tiled windows in the current workspace along with their IDs
    fn tiled_window_rects(&self) -> Vec<(WindowId, Rect)> {
        let layout = self.layout_man.get_current_layout();

        let ctx = self.make_layout_ctx();
        layout.arrange(&ctx)
    }

    /// Returns all floating windows in the current workspace along with their IDs
    fn floating_window_rects(&self) -> Vec<(WindowId, Rect)> {
        let mut vs = vec![];
        for (id, &workspace::Geometry { x, y, w, h }) in self.curr_ws().floating_windows() {
            vs.push((*id, Rect { x, y, w, h }));
        }
        vs
    }

    /// Adds the window from `event` 
    /// TODO: This function **NEEDS** refactoring
    fn add_window(&mut self, event: MapRequestEvent) {
        self.x11.map_window(event.window);
        if let Some(strut) = self.ewmh.get_strut(&mut self.x11, event.window) {
            self.struts.insert(event.window, strut);
            self.layout();
        }
        if self.ewmh.window_is_dock(&mut self.x11, event.window) {
            return;
        }
        if self.ewmh.window_is_normal(&mut self.x11, event.window) {
            _ = self
                .x11
                .conn
                .change_window_attributes(
                    event.window,
                    &ChangeWindowAttributesAux::new().event_mask(EventMask::ENTER_WINDOW),
                )
                .map_err(|e| {
                    error!(
                        "Failed to set tiled window {window} event mask: {e}",
                        window = event.window
                    )
                });
            self.set_window_border_width(event.window, self.border_width);
            self.set_window_border_pixel(event.window, self.inactive_border_color);
            self.curr_ws_mut().push_window(event.window);
            self.curr_ws_mut().set_focused_to_newest_tiled_window();
            self.layout();
            self.x11.focus_window(event.window);
        } else {
            _ = self
                .x11
                .conn
                .change_window_attributes(
                    event.window,
                    &ChangeWindowAttributesAux::new().event_mask(EventMask::ENTER_WINDOW),
                )
                .map_err(|e| {
                    error!(
                        "Failed to set floating window {window} event mask: {e}",
                        window = event.window
                    )
                });
            let (w, h) =
                x11rb::properties::WmSizeHints::get_normal_hints(&self.x11.conn, event.window)
                    .ok()
                    .and_then(|c| c.reply().ok())
                    .and_then(|h| h)
                    .and_then(|h| h.min_size)
                    .or_else(|| {
                        self.x11
                            .conn
                            .get_geometry(event.window)
                            .ok()
                            .and_then(|c| c.reply().ok())
                            .map(|g| (g.width as i32, g.height as i32))
                    })
                    .unwrap_or((200, 150));
            let (sw, sh) = self.x11.screen_size();
            let (x, y) = (
                (sw / 2) as i16 - (w / 2) as i16,
                (sh / 2) as i16 - (h / 2) as i16,
            );
            self.curr_ws_mut().push_float_window(
                event.window,
                workspace::Geometry {
                    x,
                    y,
                    w: w as i16,
                    h: h as i16,
                },
            );
            self.set_focus(event.window);
            self.set_window_border_width(event.window, self.border_width);
            self.set_window_border_pixel(event.window, self.inactive_border_color);

            self.x11.resize_window(event.window, w as u32, h as u32);
            self.x11.move_window(event.window, x, y);
            self.x11.raise_window(event.window);
            self.x11.focus_window(event.window);
        }
        self.update_lua_runtime_info();
    }

    /// Removes the window from all registries, essentially forgetting it
    fn remove_window(&mut self, event: UnmapNotifyEvent) {
        self.struts.remove(&event.window);
        self.curr_ws_mut().remove_window(event.window);
        self.layout();
        self.update_lua_runtime_info();
    }

    /// Helper function to create a layout context for the layout to use
    fn make_layout_ctx(&self) -> layout::LayoutContext<'_> {
        layout::LayoutContext {
            windows: self.curr_ws().windows(),
            screen_width: self.x11.screen_size().0,
            screen_height: self.x11.screen_size().1,
            gap: self.gap,
            reserved: self.get_reserved_space(),
            master_ratio: self.master_ratio,
        }
    }

    /// Tries to move the currently active window to be in the place of the window to the left, of course
    /// if the active layout allows that to happen
    fn swap_left(&mut self) {
        self.suppress_cursor_focus = true;
        if let Some(current) = self.curr_ws().get_focused_id() {
            let layout = self.layout_man.get_current_layout();
            let ctx = self.make_layout_ctx();

            if let Some(new_order) = layout.swap(&ctx, current, layout::Direction::Left) {
                *self.curr_ws_mut().windows_mut() = new_order;
                self.layout();
                self.refocus_and_warp(current);
            }
        }
        self.suppress_cursor_focus = false;
    }

    /// Tries to move the currently active window to be in the place of the window to the right, of course
    /// if the active layout allows that to happen
    fn swap_right(&mut self) {
        self.suppress_cursor_focus = true;
        if let Some(current) = self.curr_ws().get_focused_id() {
            let layout = self.layout_man.get_current_layout();
            let ctx = self.make_layout_ctx();

            if let Some(new_order) = layout.swap(&ctx, current, layout::Direction::Right) {
                *self.curr_ws_mut().windows_mut() = new_order;
                self.layout();
                self.refocus_and_warp(current);
            }
        }
        self.suppress_cursor_focus = false;
    }

    /// Tries to move the currently active window to be in the place of the window above, of course
    /// if the active layout allows that to happen
    fn swap_up(&mut self) {
        self.suppress_cursor_focus = true;
        if let Some(current) = self.curr_ws().get_focused_id() {
            let layout = self.layout_man.get_current_layout();
            let ctx = self.make_layout_ctx();

            if let Some(new_order) = layout.swap(&ctx, current, layout::Direction::Up) {
                *self.curr_ws_mut().windows_mut() = new_order;
                self.layout();
                self.refocus_and_warp(current);
            }
        }
        self.suppress_cursor_focus = false;
    }

    /// Tries to move the currently active window to be in the place of the window below, of course
    /// if the active layout allows that to happen
    fn swap_down(&mut self) {
        self.suppress_cursor_focus = true;
        if let Some(current) = self.curr_ws().get_focused_id() {
            let layout = self.layout_man.get_current_layout();
            let ctx = self.make_layout_ctx();

            if let Some(new_order) = layout.swap(&ctx, current, layout::Direction::Down) {
                *self.curr_ws_mut().windows_mut() = new_order;
                self.layout();
                self.refocus_and_warp(current);
            }
        }
        self.suppress_cursor_focus = false;
    }

    /// Tries to spawn the user defined commandline in a shell that should be their app launcher
    /// (dmenu_run, ...)
    fn launcher(&mut self) {
        let _ = Command::new("sh")
            .arg("-c")
            .arg(&self.launcher)
            .spawn()
            .map_err(|e| {
                warn!("Failed to launch launcher {}: {e}", &self.launcher);
            });
    }

    /// Tries to spawn the user defined commandline in a shell that should be their terminal
    fn terminal(&mut self) {
        let _ = Command::new("sh")
            .arg("-c")
            .arg(&self.terminal)
            .spawn()
            .map_err(|e| {
                warn!("Failed to launch terminal {}: {e}", &self.terminal);
            });
    }

    /// Tries to focus on the window to the left of the currently active window if possible on the currently active layout
    fn focus_left(&mut self) {
        if let Some(f_id) = self.curr_ws().get_focused_id() {
            let layout = self.layout_man.get_current_layout();
            let ctx = self.make_layout_ctx();

            if let Some(next) = layout.focus_next(&ctx, f_id, layout::Direction::Left) {
                self.curr_ws_mut().set_focused_id(next);
                self.set_focus(next);
            }
        }
    }

    /// Tries to focus on the window to the right of the currently active window if possible on the currently active layout
    fn focus_right(&mut self) {
        if let Some(f_id) = self.curr_ws().get_focused_id() {
            let layout = self.layout_man.get_current_layout();
            let ctx = self.make_layout_ctx();

            if let Some(next) = layout.focus_next(&ctx, f_id, layout::Direction::Right) {
                self.curr_ws_mut().set_focused_id(next);
                self.set_focus(next);
            }
        }
    }

    /// Tries to focus on the window above if possible on the currently active layout
    fn focus_up(&mut self) {
        if let Some(f_id) = self.curr_ws().get_focused_id() {
            let layout = self.layout_man.get_current_layout();
            let ctx = self.make_layout_ctx();

            if let Some(next) = layout.focus_next(&ctx, f_id, layout::Direction::Up) {
                self.curr_ws_mut().set_focused_id(next);
                self.set_focus(next);
            }
        }
    }

    /// Tries to focus on the window below if possible on the currently active layout
    fn focus_down(&mut self) {
        if let Some(f_id) = self.curr_ws().get_focused_id() {
            let layout = self.layout_man.get_current_layout();
            let ctx = self.make_layout_ctx();

            if let Some(next) = layout.focus_next(&ctx, f_id, layout::Direction::Down) {
                self.curr_ws_mut().set_focused_id(next);
                self.set_focus(next);
            }
        }
    }

    /// According to the current layout moves and resizes windows around
    fn layout(&mut self) {
        if self.curr_ws().empty() {
            return;
        }

        let rects = self.tiled_window_rects();

        for (w, r) in rects.iter() {
            if self.ewmh.window_is_dock(&mut self.x11, *w)
                && let Some(strut) = self.ewmh.get_strut(&mut self.x11, *w)
            {
                self.struts.insert(*w, strut);
                continue;
            }
            self.x11.move_window(*w, r.x, r.y);
            self.x11.resize_window(*w, r.w as u32, r.h as u32);
        }
    }

    /// Focuses the x11 context to the `id` window ID, sets the last focused windows border to be
    /// inactive and sets the `id` windows border to be active
    fn set_focus(&mut self, id: WindowId) {
        if let Some(prev) = self.last_focused {
            self.set_window_border_pixel(prev, self.inactive_border_color);
        }

        self.set_window_border_pixel(id, self.active_border_color);
        let _ = self.x11.focus_window(id);

        self.curr_ws_mut().set_focused_id(id);
        self.last_focused = Some(id);
        self.update_lua_runtime_info();
    }
}

fn main() -> Result<(), ()> {
    let display_name =
        std::env::var("DISPLAY").map_err(|e| error!("Failed to get $DISPLAY. Aborting: {e}"))?;
    match Nwm::create(&display_name) {
        Some(nwm) => nwm.run(),
        None => {
            error!("Upsi dupsi, failed to create nwm :(");
        }
    }
    Ok(())
}


/// Every single `nwm.bind` call results in this struct begin created
#[derive(Clone)]
struct Bind {
    /// Can be either a native action or a Lua callback
    action: lua_cfg::BindAction,
    bind: lua_cfg::KeyCombo,
}

/// Combines user set prefixes in `nwm.bind` (that is not including the last key)
fn keycombo_mask(kc: &lua_cfg::KeyCombo) -> u16 {
    let mut mask = 0;
    for m in &kc.prefixes {
        mask |= match m {
            lua_cfg::SpecialKey::Shift => ModMask::SHIFT,
            lua_cfg::SpecialKey::Control => ModMask::CONTROL,
            lua_cfg::SpecialKey::Alt => ModMask::M1,
            lua_cfg::SpecialKey::Super => ModMask::M4,
        };
    }
    mask
}

impl Bind {
    /// Tries to do the keybind set by `nwm.bind` in the users config
    fn try_do(&self, nwm: &mut Nwm, ev: KeyPressEvent) {
        let want_keycode = nwm.x11.key_to_keycode(self.bind.key.into_x11rb());

        if ev.detail as u32 != want_keycode {
            return;
        }

        let want_mask = keycombo_mask(&self.bind);
        let actual_mask = ev.state & !(ModMask::M2 | ModMask::LOCK).bits();

        if actual_mask.bits() != want_mask {
            return;
        }

        match &self.action {
            lua_cfg::BindAction::Native(action) => {
                let action_fn = action_to_fn(*action);
                action_fn(nwm);
            }
            lua_cfg::BindAction::LuaCallback(callback) => {
                if let Some(lua) = &nwm.lua {
                    if let Ok(func) = lua.registry_value::<mlua::Function>(&callback.key) {
                        if let Err(e) = func.call::<()>(()) {
                            warn!("Lua callback error: {}", e);
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
/// Used for window rectangles (tiled/floating)
struct Rect {
    x: i16,
    y: i16,
    w: i16,
    h: i16,
}

#[derive(Debug, Clone, Copy, Default)]
/// Used for deriving the reserved space by bars (polybar, ...)
struct Reserve {
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
}

use x11rb::protocol::{
    Event,
    xproto::{
        ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt, EventMask, KeyPressEvent,
        MapRequestEvent, ModMask, UnmapNotifyEvent,
    },
};


/// Maps `nwm.action<action>` to nwm functions or even lambdas
fn action_to_fn(action: lua_cfg::Action) -> fn(&mut Nwm) {
    match action {
        lua_cfg::Action::FocusLeft => Nwm::focus_left,
        lua_cfg::Action::FocusRight => Nwm::focus_right,
        lua_cfg::Action::MoveLeft => Nwm::swap_left,
        lua_cfg::Action::MoveRight => Nwm::swap_right,

        lua_cfg::Action::FocusUp => Nwm::focus_up,
        lua_cfg::Action::FocusDown => Nwm::focus_down,

        lua_cfg::Action::MoveUp => Nwm::swap_up,
        lua_cfg::Action::MoveDown => Nwm::swap_down,

        lua_cfg::Action::Launcher => Nwm::launcher,
        lua_cfg::Action::Terminal => Nwm::terminal,
        lua_cfg::Action::CloseWindow => Nwm::close_focused,
        lua_cfg::Action::NextWs => Nwm::focus_next_ws,
        lua_cfg::Action::PrevWs => Nwm::focus_prev_ws,
        lua_cfg::Action::ReloadConfig => Nwm::reload_config,
        lua_cfg::Action::Ws0 => |nwm: &mut Nwm| {
            nwm.switch_ws(0);
        },
        lua_cfg::Action::Ws1 => |nwm: &mut Nwm| {
            nwm.switch_ws(1);
        },
        lua_cfg::Action::Ws2 => |nwm: &mut Nwm| {
            nwm.switch_ws(2);
        },
        lua_cfg::Action::Ws3 => |nwm: &mut Nwm| {
            nwm.switch_ws(3);
        },
        lua_cfg::Action::Ws4 => |nwm: &mut Nwm| {
            nwm.switch_ws(4);
        },
        lua_cfg::Action::Ws5 => |nwm: &mut Nwm| {
            nwm.switch_ws(5);
        },
        lua_cfg::Action::Ws6 => |nwm: &mut Nwm| {
            nwm.switch_ws(6);
        },
        lua_cfg::Action::Ws7 => |nwm: &mut Nwm| {
            nwm.switch_ws(7);
        },
        lua_cfg::Action::Ws8 => |nwm: &mut Nwm| {
            nwm.switch_ws(8);
        },
        lua_cfg::Action::Ws9 => |nwm: &mut Nwm| {
            nwm.switch_ws(9);
        },
        lua_cfg::Action::MoveToWs0 => |nwm: &mut Nwm| {
            nwm.move_focused_to_ws(0);
        },
        lua_cfg::Action::MoveToWs1 => |nwm: &mut Nwm| {
            nwm.move_focused_to_ws(1);
        },
        lua_cfg::Action::MoveToWs2 => |nwm: &mut Nwm| {
            nwm.move_focused_to_ws(2);
        },
        lua_cfg::Action::MoveToWs3 => |nwm: &mut Nwm| {
            nwm.move_focused_to_ws(3);
        },
        lua_cfg::Action::MoveToWs4 => |nwm: &mut Nwm| {
            nwm.move_focused_to_ws(4);
        },
        lua_cfg::Action::MoveToWs5 => |nwm: &mut Nwm| {
            nwm.move_focused_to_ws(5);
        },
        lua_cfg::Action::MoveToWs6 => |nwm: &mut Nwm| {
            nwm.move_focused_to_ws(6);
        },
        lua_cfg::Action::MoveToWs7 => |nwm: &mut Nwm| {
            nwm.move_focused_to_ws(7);
        },
        lua_cfg::Action::MoveToWs8 => |nwm: &mut Nwm| {
            nwm.move_focused_to_ws(8);
        },
        lua_cfg::Action::MoveToWs9 => |nwm: &mut Nwm| {
            nwm.move_focused_to_ws(9);
        },
        lua_cfg::Action::Quit => |nwm: &mut Nwm| {
            nwm.running = false;
        },
        lua_cfg::Action::NextLayout => |nwm: &mut Nwm| {
            nwm.layout_man.next_layout();
            nwm.layout();
        },
        lua_cfg::Action::PrevLayout => |nwm: &mut Nwm| {
            nwm.layout_man.prev_layout();
            nwm.layout();
        },
        lua_cfg::Action::GapUp => |nwm: &mut Nwm| {
            nwm.gap += 1;
            nwm.layout();
        },
        lua_cfg::Action::GapDown => |nwm: &mut Nwm| {
            nwm.gap = nwm.gap.saturating_sub(1);
            nwm.layout();
        },
        lua_cfg::Action::MasterRatioDown => |nwm: &mut Nwm| {
            nwm.master_ratio = (nwm.master_ratio - 0.1).clamp(0.1, 0.9);
            nwm.layout();
        },
        lua_cfg::Action::MasterRatioUp => |nwm: &mut Nwm| {
            nwm.master_ratio = (nwm.master_ratio + 0.1).clamp(0.1, 0.9);
            nwm.layout();
        },
    }
}
