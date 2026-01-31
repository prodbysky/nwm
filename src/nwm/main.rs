mod better_x11rb;
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
    last_x: i16,
    last_y: i16,
    window_type_atom: Option<Atom>,
    window_type_normal_atom: Option<Atom>,
    window_type_dock_atom: Option<Atom>,
    strut_partial_atom: Option<Atom>,
    active_desktop_atom: Option<Atom>,
    state_atom: Option<Atom>,
    fullscreen_state_atom: Option<Atom>,
    struts: HashMap<WindowId, Strut>,

    gap: u8,
    binds: Vec<Bind>,
    terminal: String,
    launcher: String,
    border_width: u8,
    active_border_color: u32,
    inactive_border_color: u32,
    suppress_cursor_focus: bool,
    layouts: Vec<Box<dyn layout::Layout>>,
    curr_layout: usize,
}

#[allow(dead_code)]
struct Strut {
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,

    left_start_y: u32,
    left_end_y: u32,
    right_start_y: u32,
    right_end_y: u32,
    top_start_x: u32,
    top_end_x: u32,
    bottom_start_x: u32,
    bottom_end_x: u32,
}

impl From<[u32; 12]> for Strut {
    fn from(value: [u32; 12]) -> Self {
        Strut {
            left: value[0],
            right: value[1],
            top: value[2],
            bottom: value[3],
            left_start_y: value[4],
            left_end_y: value[5],
            right_start_y: value[6],
            right_end_y: value[7],
            top_start_x: value[8],
            top_end_x: value[9],
            bottom_start_x: value[10],
            bottom_end_x: value[11],
        }
    }
}

#[derive(Debug, Clone)]
struct Bind {
    action: fn(&mut Nwm),
    bind: lua_cfg::KeyCombo,
}

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

        (self.action)(nwm);
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Rect {
    x: i16,
    y: i16,
    w: i16,
    h: i16,
}

#[derive(Debug, Clone, Copy, Default)]
struct Reserve {
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
}

use x11rb::{
    protocol::{
        Event,
        xproto::{
            Atom, AtomEnum, ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt,
            EventMask, KeyPressEvent, MapRequestEvent, ModMask, PropMode, UnmapNotifyEvent,
        },
    },
    wrapper::ConnectionExt as OtherConnExt,
};

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
            nwm.curr_layout = (nwm.curr_layout + 1) % nwm.layouts.len();
            nwm.layout();
        },
        lua_cfg::Action::PrevLayout => |nwm: &mut Nwm| {
            match nwm.curr_layout {
                0 => nwm.curr_layout = nwm.layouts.len() - 1,
                _ => {
                    nwm.curr_layout = nwm.curr_layout - 1;
                }
            };
            nwm.layout();
        },
    }
}

impl Nwm {
    fn apply_lua_config(
        conf: lua_cfg::Config,
        x11: &mut better_x11rb::X11RB,
    ) -> (u8, Vec<Bind>, String, String, u32, u32, u8) {
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
                action: action_to_fn(b.action),
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
        )
    }

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
    }

    fn reload_config(&mut self) {
        let conf = match lua_cfg::load_config(true) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to reload lua config: {e:?}");
                return;
            }
        };

        self.binds.clear();

        let (gap, binds, terminal, launcher, active, inactive, width) =
            Self::apply_lua_config(conf, &mut self.x11);

        self.gap = gap;
        self.binds = binds;
        self.terminal = terminal;
        self.launcher = launcher;
        self.active_border_color = active;
        self.inactive_border_color = inactive;
        self.border_width = width;

        for ws in self.workspaces.clone() {
            for w in ws.windows() {
                self.set_window_border_width(*w, self.border_width);
            }
            for w in ws.floating_window_ids() {
                self.set_window_border_width(*w, self.border_width);
            }
        }

        info!("Reloaded lua config");
    }

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
        let (gap, binds, terminal, launcher, active, inactive, width) =
            Self::apply_lua_config(conf, &mut x11_ab);

        info!("Everything went well in initialization :DD");
        if launcher.is_empty() {
            warn!("Launcher wasn't set to a program");
        }
        if terminal.is_empty() {
            warn!("Terminal wasn't set to a program");
        }

        let window_type_atom = x11_ab.intern_atom(b"_NET_WM_WINDOW_TYPE");
        if window_type_atom.is_none() {
            warn!("Failed to intern _NET_WM_WINDOW_TYPE, emwh window type support is not present");
        }
        let window_type_dock_atom = x11_ab.intern_atom(b"_NET_WM_WINDOW_TYPE_DOCK");

        if window_type_dock_atom.is_none() {
            warn!(
                "Failed to intern _NET_WM_WINDOW_TYPE_DOCK, emwh window type support is not present"
            );
        }

        let window_type_normal_atom = x11_ab.intern_atom(b"_NET_WM_WINDOW_TYPE_NORMAL");
        if window_type_normal_atom.is_none() {
            warn!(
                "Failed to intern _NET_WM_WINDOW_TYPE_NORMAL, emwh window type support is not present"
            );
        }
        let strut_partial_atom = x11_ab.intern_atom(b"_NET_WM_STRUT_PARTIAL");
        if strut_partial_atom.is_none() {
            warn!(
                "Failed to intern _NET_WM_STRUT_PARTIAL, docks that depend on this won't resize other windows"
            );
        }

        let fullscreen_state_atom = x11_ab.intern_atom(b"_NET_WM_STATE_FULLSCREEN");
        if fullscreen_state_atom.is_none() {
            warn!(
                "Failed to intern _NET_WM_STATE_FULLSCREEN, fullscreen state will not be handled"
            );
        }

        let state_atom = x11_ab.intern_atom(b"_NET_WM_STATE");
        if state_atom.is_none() {
            warn!("Failed to intern _NET_WM_STATE, fullscreen state will not be handled");
        }
        use x11rb::wrapper::ConnectionExt;

        let active_desktop_atom = x11_ab.intern_atom(b"_NET_CURRENT_DESKTOP");
        if let Some(at) = active_desktop_atom {
            _ = x11_ab
                .conn
                .change_property32(
                    PropMode::REPLACE,
                    x11_ab.root_window(),
                    at,
                    AtomEnum::CARDINAL,
                    &[10],
                )
                .map_err(|e| {
                    warn!("Failed to set _NET_CURRENT_DESKTOP: {e}");
                });
        }

        let layouts: Vec<Box<dyn layout::Layout>> = vec![
            Box::new(layout::HorizontalTiling),
            Box::new(layout::VerticalTiling),
            Box::new(layout::MasterLayout),
        ];

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
            window_type_atom,
            window_type_dock_atom,
            strut_partial_atom,
            active_desktop_atom,
            window_type_normal_atom,
            state_atom,
            fullscreen_state_atom,
            struts: HashMap::new(),
            last_focused: None,
            active_border_color: active,
            inactive_border_color: inactive,
            border_width: width,
            suppress_cursor_focus: false,
            layouts,
            curr_layout: 0,
        })
    }
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

    fn get_window_type(&self, w: WindowId, atom: Atom) -> Option<Vec<Atom>> {
        let rep = self
            .x11
            .conn
            .get_property(false, w, atom, AtomEnum::ATOM, 0, 32)
            .unwrap()
            .reply()
            .map_err(|e| {
                warn!("Failed to get reply from getting the window type of window {w}: {e}")
            })
            .ok()?;

        if rep.format != 32 {
            return None;
        }

        Some(rep.value32().unwrap().collect())
    }

    fn get_strut_partial(&self, w: WindowId, atom: Atom) -> Option<[u32; 12]> {
        let rep = self
            .x11
            .conn
            .get_property(false, w, atom, AtomEnum::CARDINAL, 0, 12)
            .map_err(|e| {
                warn!("Failed to get _NET_WM_STRUT_PARTIAL property: {e}");
            })
            .ok()?
            .reply()
            .map_err(|e| {
                warn!(
                    "Failed to get _NET_WM_STRUT_PARTIAL property reply from the x11 server: {e}"
                );
            })
            .ok()?;

        let values = rep.value32()?.collect::<Vec<_>>();

        if values.len() < 12 {
            return None;
        }

        let mut arr = [0u32; 12];

        arr.copy_from_slice(&values[..12]);
        Some(arr)
    }

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

    fn focus_next_ws(&mut self) {
        self.switch_ws((self.curr_workspace + 1).clamp(0, 9));
    }

    fn focus_prev_ws(&mut self) {
        if self.curr_workspace == 0 {
            return;
        }
        self.switch_ws((self.curr_workspace - 1).clamp(0, 9));
    }

    fn focused(&self) -> Option<WindowId> {
        self.workspaces[self.curr_workspace].get_focused_id()
    }

    fn close_focused(&mut self) {
        if let Some(w) = self.focused() {
            self.close_window(w);
        }
    }

    fn close_window(&mut self, id: WindowId) {
        self.x11.close_window(id);
        // self.curr_ws_mut().remove_window(id);
    }

    fn curr_ws_mut(&mut self) -> &mut workspace::Workspace {
        &mut self.workspaces[self.curr_workspace]
    }

    fn curr_ws(&self) -> &workspace::Workspace {
        &self.workspaces[self.curr_workspace]
    }

    fn set_fullscreen(&mut self, id: WindowId) {
        self.curr_ws_mut().set_fullscreen_id(Some(id));
        for w in self.curr_ws().windows().iter().filter(|w_id| **w_id != id) {
            _ = self.x11.conn.unmap_window(*w);
        }
        let (sw, sh) = self.x11.screen_size();
        self.x11.move_window(id, 0, 0);
        self.x11.resize_window(id, sw as u32, sh as u32);
    }

    fn unset_fullscreen(&mut self) {
        if let Some(id) = self.curr_ws().get_fullscreen_id() {
            for w in self.curr_ws().windows().iter().filter(|w_id| **w_id != id) {
                _ = self.x11.conn.map_window(*w);
            }
        }
        self.layout();
        self.curr_ws_mut().set_fullscreen_id(None);
    }

    pub fn run(mut self) {
        info!("Keybindings were setup");

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
                    if let Some(sa) = self.state_atom
                        && let Some(fsa) = self.fullscreen_state_atom
                    {
                        if e.type_ == sa {
                            let (action, first, second) = (
                                e.data.as_data32()[0],
                                e.data.as_data32()[1],
                                e.data.as_data32()[2],
                            );
                            if first == fsa || second == fsa {
                                match action {
                                    0 => {
                                        self.unset_fullscreen();
                                    }
                                    1 => {
                                        self.set_fullscreen(e.window);
                                    }
                                    2 => {
                                        dbg!("toggle_fs");
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Event::PropertyNotify(e) => {
                    if let Some(spa) = self.strut_partial_atom
                        && e.atom == spa
                        && let Some(strut) = self.get_strut_partial(e.window, spa)
                    {
                        self.struts.insert(e.window, Strut::from(strut));
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

        if let Some(ada) = self.active_desktop_atom {
            _ = self
                .x11
                .conn
                .change_property32(
                    PropMode::REPLACE,
                    self.x11.root_window(),
                    ada,
                    AtomEnum::CARDINAL,
                    &[(new_ws) as u32],
                )
                .map_err(|e| {
                    warn!("Failed to set _NET_ACTIVE_DESKTOP: {e}");
                });
        }

        self.layout();
        self.focus_on_pointer();
    }

    fn tiled_window_rects(&self) -> Vec<(WindowId, Rect)> {
        let layout = &self.layouts[self.curr_layout];

        let ctx = self.make_layout_ctx();
        layout.arrange(&ctx)
    }

    fn floating_window_rects(&self) -> Vec<(WindowId, Rect)> {
        let mut vs = vec![];
        for (id, &workspace::Geometry { x, y, w, h }) in self.curr_ws().floating_windows() {
            vs.push((*id, Rect { x, y, w, h }));
        }
        vs
    }

    fn window_is_dock(&self, w: WindowId) -> bool {
        if let Some(wta) = self.window_type_atom
            && let Some(wtda) = self.window_type_dock_atom
            && let Some(types) = self.get_window_type(w, wta)
            && types.contains(&wtda)
        {
            return true;
        }
        false
    }

    fn window_is_normal(&self, w: WindowId) -> bool {
        if let Some(wta) = self.window_type_atom
            && let Some(wtna) = self.window_type_normal_atom
            && let Some(types) = self.get_window_type(w, wta)
            && types.contains(&wtna)
        {
            return true;
        }
        false
    }

    fn add_window(&mut self, event: MapRequestEvent) {
        self.x11.map_window(event.window);
        if let Some(spa) = self.strut_partial_atom
            && let Some(strut) = self.get_strut_partial(event.window, spa)
        {
            self.struts.insert(event.window, Strut::from(strut));
            self.layout();
        }
        if self.window_is_dock(event.window) {
            return;
        }
        if self.window_is_normal(event.window) {
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
    }

    fn remove_window(&mut self, event: UnmapNotifyEvent) {
        self.struts.remove(&event.window);
        self.curr_ws_mut().remove_window(event.window);
        self.layout();
    }

    fn make_layout_ctx(&self) -> layout::LayoutContext<'_> {
        layout::LayoutContext {
            windows: self.curr_ws().windows(),
            screen_width: self.x11.screen_size().0,
            screen_height: self.x11.screen_size().1,
            gap: self.gap,
            reserved: self.get_reserved_space(),
        }
    }

    fn swap_left(&mut self) {
        self.suppress_cursor_focus = true;
        if let Some(current) = self.curr_ws().get_focused_id() {
            let layout = &self.layouts[self.curr_layout];
            let ctx = self.make_layout_ctx();

            if let Some(new_order) = layout.swap(&ctx, current, layout::Direction::Left) {
                *self.curr_ws_mut().windows_mut() = new_order;
                self.layout();
                self.refocus_and_warp(current);
            }
        }
        self.suppress_cursor_focus = false;
    }

    fn swap_right(&mut self) {
        self.suppress_cursor_focus = true;
        if let Some(current) = self.curr_ws().get_focused_id() {
            let layout = &self.layouts[self.curr_layout];
            let ctx = self.make_layout_ctx();

            if let Some(new_order) = layout.swap(&ctx, current, layout::Direction::Right) {
                *self.curr_ws_mut().windows_mut() = new_order;
                self.layout();
                self.refocus_and_warp(current);
            }
        }
        self.suppress_cursor_focus = false;
    }

    fn swap_up(&mut self) {
        self.suppress_cursor_focus = true;
        if let Some(current) = self.curr_ws().get_focused_id() {
            let layout = &self.layouts[self.curr_layout];
            let ctx = self.make_layout_ctx();

            if let Some(new_order) = layout.swap(&ctx, current, layout::Direction::Up) {
                *self.curr_ws_mut().windows_mut() = new_order;
                self.layout();
                self.refocus_and_warp(current);
            }
        }
        self.suppress_cursor_focus = false;
    }

    fn swap_down(&mut self) {
        self.suppress_cursor_focus = true;
        if let Some(current) = self.curr_ws().get_focused_id() {
            let layout = &self.layouts[self.curr_layout];
            let ctx = self.make_layout_ctx();

            if let Some(new_order) = layout.swap(&ctx, current, layout::Direction::Down) {
                *self.curr_ws_mut().windows_mut() = new_order;
                self.layout();
                self.refocus_and_warp(current);
            }
        }
        self.suppress_cursor_focus = false;
    }

    fn launcher(&mut self) {
        let _ = Command::new("sh")
            .arg("-c")
            .arg(&self.launcher)
            .spawn()
            .map_err(|e| {
                warn!("Failed to launch launcher {}: {e}", &self.launcher);
            });
    }

    fn terminal(&mut self) {
        let _ = Command::new("sh")
            .arg("-c")
            .arg(&self.terminal)
            .spawn()
            .map_err(|e| {
                warn!("Failed to launch terminal {}: {e}", &self.terminal);
            });
    }

    fn focus_left(&mut self) {
        if let Some(f_id) = self.curr_ws().get_focused_id() {
            let layout = &self.layouts[self.curr_layout];
            let ctx = self.make_layout_ctx();

            if let Some(next) = layout.focus_next(&ctx, f_id, layout::Direction::Left) {
                self.curr_ws_mut().set_focused_id(next);
                self.set_focus(next);
            }
        }
    }

    fn focus_right(&mut self) {
        if let Some(f_id) = self.curr_ws().get_focused_id() {
            let layout = &self.layouts[self.curr_layout];
            let ctx = self.make_layout_ctx();

            if let Some(next) = layout.focus_next(&ctx, f_id, layout::Direction::Right) {
                self.curr_ws_mut().set_focused_id(next);
                self.set_focus(next);
            }
        }
    }

    fn focus_up(&mut self) {
        if let Some(f_id) = self.curr_ws().get_focused_id() {
            let layout = &self.layouts[self.curr_layout];
            let ctx = self.make_layout_ctx();

            if let Some(next) = layout.focus_next(&ctx, f_id, layout::Direction::Up) {
                self.curr_ws_mut().set_focused_id(next);
                self.set_focus(next);
            }
        }
    }

    fn focus_down(&mut self) {
        if let Some(f_id) = self.curr_ws().get_focused_id() {
            let layout = &self.layouts[self.curr_layout];
            let ctx = self.make_layout_ctx();

            if let Some(next) = layout.focus_next(&ctx, f_id, layout::Direction::Down) {
                self.curr_ws_mut().set_focused_id(next);
                self.set_focus(next);
            }
        }
    }

    fn layout(&mut self) {
        if self.curr_ws().empty() {
            return;
        }

        let rects = self.tiled_window_rects();

        for (w, r) in rects.iter() {
            if self.window_is_dock(*w)
                && let Some(spa) = self.strut_partial_atom
                && let Some(strut) = self.get_strut_partial(*w, spa)
            {
                self.struts.insert(
                    *w,
                    Strut {
                        left: strut[0],
                        right: strut[1],
                        top: strut[2],
                        bottom: strut[3],
                        left_start_y: strut[4],
                        left_end_y: strut[5],
                        right_start_y: strut[6],
                        right_end_y: strut[7],
                        top_start_x: strut[8],
                        top_end_x: strut[9],
                        bottom_start_x: strut[10],
                        bottom_end_x: strut[11],
                    },
                );
                continue;
            }
            self.x11.move_window(*w, r.x, r.y);
            self.x11.resize_window(*w, r.w as u32, r.h as u32);
        }
    }

    fn set_focus(&mut self, id: WindowId) {
        if let Some(prev) = self.last_focused {
            self.set_window_border_pixel(prev, self.inactive_border_color);
        }

        self.set_window_border_pixel(id, self.active_border_color);
        let _ = self.x11.focus_window(id);

        self.curr_ws_mut().set_focused_id(id);
        self.last_focused = Some(id);
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
