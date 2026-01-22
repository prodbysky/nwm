mod better_x11rb;
mod multi_log;
mod lua_cfg;
use colored::Colorize;

use std::{collections::HashMap, io::Write, process::Command};

use better_x11rb::WindowId;

use log::{Level, info, warn};

struct Nwm {
    x11: better_x11rb::X11RB,
    workspaces: [Vec<WindowId>; 10],
    curr_workspace: usize,
    focused: [Option<usize>; 10],
    last_focused: Option<WindowId>,
    running: bool,
    last_x: i16,
    last_y: i16,
    gap: u8,
    binds: Vec<Bind>,
    terminal: String,
    launcher: String,
    window_type_atom: Option<Atom>,
    window_type_dock_atom: Option<Atom>,
    strut_partial_atom: Option<Atom>,
    active_desktop_atom: Option<Atom>,
    struts: HashMap<WindowId, Strut>,
    border_width: u8,
    active_border_color: u32,
    inactive_border_color: u32,
    config_path: std::path::PathBuf,
}

use std::sync::Mutex;

struct NwLogLog {
    out: Mutex<std::fs::File>,
}

impl NwLogLog {
    pub fn init(stdin: std::fs::File) -> Self {
        Self {
            out: Mutex::new(stdin),
        }
    }
}

impl log::Log for NwLogLog {
    fn flush(&self) {
        if let Ok(mut stdin) = self.out.lock() {
            let _ = stdin.flush();
        }
    }
    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        if let Ok(mut stdin) = self.out.lock() {
            let _ = writeln!(
                stdin,
                "{} -> {}",
                record.level().as_str().yellow(),
                record.args()
            );
        }
    }

    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= Level::Info
    }
}

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
            _ => ModMask::default(),
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
    connection::Connection,
    protocol::{
        Event,
        xproto::{
            Atom, AtomEnum, ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt,
            KeyPressEvent, MapRequestEvent, ModMask, PropMode, UnmapNotifyEvent,
        },
    },
    wrapper::ConnectionExt as OtherConnExt,
};

#[derive(Clone, Copy)]
enum MasterKey {
    Super,
    Shift,
    Control,
    Alt,
}

fn action_to_fn(action: lua_cfg::Action) -> fn(&mut Nwm) {
    match action {
        lua_cfg::Action::FocusLeft => Nwm::focus_left,
        lua_cfg::Action::FocusRight => Nwm::focus_right,
        lua_cfg::Action::MoveLeft => Nwm::swap_left,
        lua_cfg::Action::MoveRight => Nwm::swap_right,
        lua_cfg::Action::Launcher => Nwm::launcher,
        lua_cfg::Action::Terminal => Nwm::terminal,
        lua_cfg::Action::CloseWindow => Nwm::close_focused,
        lua_cfg::Action::NextWs => Nwm::focus_next_ws,
        lua_cfg::Action::PrevWs => Nwm::focus_prev_ws,
        lua_cfg::Action::ReloadConfig => Nwm::reload_config,
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
                    lua_cfg::SpecialKey::Space => unreachable!(),
                })
                .fold(ModMask::default(), |acc, it| acc | it);

            x11.grab_key(mask, b.combo.key.into_x11rb()).unwrap();

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

    fn reload_config(&mut self) {
        let conf = match lua_cfg::load_config(&self.config_path, true) {
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

        for ws in self.workspaces.clone(){
            for w in ws {
                self.set_window_border_width(w, self.border_width);
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
                Box::new(NwLogLog::init(file)),
            ],
            log::Level::Trace,
        );
        let mut x11_ab = better_x11rb::X11RB::init()?;

        x11_ab.grab_pointer()?;

        info!("Succesfully initialized display {} ", display_name);

        let dirs = platform_dirs::AppDirs::new(Some("nwm"), false).unwrap();
        _ = std::fs::create_dir(&dirs.config_dir);
        let mut conf_dir = dirs.config_dir.clone();
        conf_dir.push("config.lua");

        let conf = lua_cfg::load_config(&conf_dir, false).unwrap();
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
        let strut_partial_atom = x11_ab.intern_atom(b"_NET_WM_STRUT_PARTIAL");
        if strut_partial_atom.is_none() {
            warn!(
                "Failed to intern _NET_WM_STRUT_PARTIAL, docks that depend on this won't resize other windows"
            );
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

        Some(Self {
            x11: x11_ab,
            workspaces: Default::default(),
            curr_workspace: 0,
            focused: Default::default(),
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
            struts: HashMap::new(),
            last_focused: None,
            active_border_color: active,
            inactive_border_color: inactive,
            border_width: width,
            config_path: conf_dir,
        })
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
            .unwrap()
            .reply()
            .unwrap();

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
        self.switch_ws((self.curr_workspace + 1).clamp(0, 10));
    }

    fn focus_prev_ws(&mut self) {
        if self.curr_workspace == 0 {
            return;
        }
        self.switch_ws((self.curr_workspace - 1).clamp(0, 10));
    }

    fn focused(&self) -> Option<usize> {
        self.focused[self.curr_workspace]
    }

    fn close_focused(&mut self) {
        if let Some(w) = self.focused() {
            self.close_window(w);
        }
    }

    fn close_window(&mut self, window_index: usize) {
        if window_index < self.curr_ws().len() {
            self.x11.close_window(self.curr_ws()[window_index]).unwrap();
            self.curr_ws_mut().remove(window_index);
        }
    }

    fn curr_ws_mut(&mut self) -> &mut Vec<WindowId> {
        &mut self.workspaces[self.curr_workspace]
    }
    fn curr_ws(&self) -> &Vec<WindowId> {
        &self.workspaces[self.curr_workspace]
    }

    pub fn run(mut self) {
        info!("Keybindings were setup");

        while self.running {
            let event = self.x11.next_event().unwrap();

            match event {
                Event::MapRequest(e) => self.add_window(e),
                Event::UnmapNotify(e) => self.remove_window(e),
                Event::KeyPress(e) => {
                    for b in &self.binds.clone() {
                        b.try_do(&mut self, e);
                    }
                }
                Event::MotionNotify(_) => {
                    let (x, y) = self.x11.mouse_pos();
                    if self.last_x != x || self.last_y != y {
                        let rects = self.window_rects();
                        for (i, r) in rects.iter().enumerate() {
                            if x > r.x && x < r.x + r.w {
                                self.focused[self.curr_workspace] = Some(i);
                                self.set_focus(i);
                            }
                        }
                        self.last_x = x;
                        self.last_y = y;
                    }
                }
                Event::KeyRelease(_) => {}
                Event::MappingNotify(_) => {}
                Event::ConfigureRequest(_) => self.layout(),
                Event::PropertyNotify(e) => {
                    if self.strut_partial_atom.is_none() {
                        continue;
                    }
                    let spa = self.strut_partial_atom.unwrap();
                    if e.atom == spa {
                        if let Some(strut) = self.get_strut_partial(e.window, spa) {
                            self.struts.insert(e.window, Strut::from(strut));
                            self.layout();
                        }
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
        let rects = self.window_rects();
        for (i, r) in rects.iter().enumerate() {
            if self.last_x > r.x && self.last_x < r.x + r.w {
                self.set_window_border_pixel(self.curr_ws()[self.focused().unwrap()], self.inactive_border_color);
                self.focused[self.curr_workspace] = Some(i);
                self.set_focus(i);
            }
        }
    }

    fn set_window_border_pixel(&mut self, w: WindowId, color: u32) {
        _ = self.x11
            .conn
            .change_window_attributes(
                w,
                &ChangeWindowAttributesAux::new().border_pixel(color),
            );
    }

    fn set_window_border_width(&mut self, w: WindowId, width: u8) {
        self.x11
            .conn
            .configure_window(
                w,
                &ConfigureWindowAux::new().border_width(width as u32),
            )
            .unwrap();
    }

    fn switch_ws(&mut self, new_ws: usize) {
        if new_ws >= self.workspaces.len() || new_ws == self.curr_workspace {
            return;
        }

        let old_ws = self.curr_workspace;

        for &w in &self.workspaces[old_ws] {
            self.x11.unmap_window(w).unwrap();
        }

        self.curr_workspace = new_ws;

        for &w in &self.workspaces[new_ws] {
            self.x11.map_window(w).unwrap();
        }

        if let Some(ada) = self.active_desktop_atom {
            self.x11
                .conn
                .change_property32(
                    PropMode::REPLACE,
                    self.x11.root_window(),
                    ada,
                    AtomEnum::CARDINAL,
                    &[(new_ws) as u32],
                )
                .unwrap();
        }
        self.x11.conn.flush().unwrap();

        self.layout();
        self.focus_on_pointer();
    }

    fn window_rects(&self) -> Vec<Rect> {
        if self.curr_ws().is_empty() {
            return vec![];
        }

        let mut rs = vec![];
        let (mut sw, mut sh) = self.x11.screen_size();

        let reserved = self.get_reserved_space();

        let offset = (reserved.x0, reserved.y0);

        sw -= (reserved.x0 + reserved.x1) as u16;
        sh -= (reserved.y0 + reserved.y1) as u16;

        let n = (self.curr_ws().len()) as i16;
        if n == 0 {
            return rs;
        }

        let gap = self.gap as i16;
        let half_gap = gap / 2;

        let usable_w = sw as i16 - gap * 2;
        let slot_w = usable_w / n;

        for i in 0..n {
            let x = gap + i * slot_w + half_gap + offset.0 as i16;
            let y = gap + offset.1 as i16;

            let w = slot_w - half_gap * 2;
            let h = sh as i16 - gap * 2;

            if w > 0 && h > 0 {
                rs.push(Rect { x, y, w, h });
            }
        }

        rs
    }

    fn window_is_dock(&self, w: WindowId) -> bool {
        if let Some(wta) = self.window_type_atom
            && let Some(wtda) = self.window_type_dock_atom
        {
            if let Some(types) = self.get_window_type(w, wta) {
                if types.contains(&wtda) {
                    return true;
                }
            }
        }
        return false;
    }

    fn add_window(&mut self, event: MapRequestEvent) {
        self.x11.map_window(event.window).unwrap();
        if let Some(spa) = self.strut_partial_atom {
            if let Some(strut) = self.get_strut_partial(event.window, spa) {
                self.struts.insert(event.window, Strut::from(strut));
                self.layout();
            }
        }
        if self.window_is_dock(event.window) {
            return;
        }
        self.set_window_border_width(event.window, self.border_width);
        self.set_window_border_pixel(event.window, self.inactive_border_color);
        self.curr_ws_mut().push(event.window);
        self.focused[self.curr_workspace] = Some(self.curr_ws().len() - 1);
        self.layout();
        self.x11.raise_window(event.window);
        self.x11.focus_window(event.window);
    }

    fn remove_window(&mut self, event: UnmapNotifyEvent) {
        if let Some(pos) = self.curr_ws().iter().position(|&w| w == event.window) {
            self.curr_ws_mut().remove(pos);
            self.struts.remove(&event.window);
            if let Some(f) = self.focused() {
                if f >= self.curr_ws().len() {
                    self.focused[self.curr_workspace] = self.curr_ws().len().checked_sub(1);
                }
            }
        }
        self.layout();
    }

    fn swap_left(&mut self) {
        if let Some(i) = self.focused() {
            if i > 0 && self.curr_ws().len() > i {
                self.curr_ws_mut().swap(i, i - 1);
                self.focused[self.curr_workspace] = Some(i - 1);
                self.layout();
            }
        }
    }

    fn swap_right(&mut self) {
        if let Some(i) = self.focused() {
            if i + 1 < self.curr_ws().len() {
                self.curr_ws_mut().swap(i, i + 1);
                self.focused[self.curr_workspace] = Some(i + 1);
                self.layout();
            }
        }
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
        if let Some(i) = self.focused[self.curr_workspace] {
            let new = i.saturating_sub(1);
            self.focused[self.curr_workspace] = Some(new);
            self.set_focus(new);
        }
    }
    fn focus_right(&mut self) {
        if let Some(i) = self.focused[self.curr_workspace] {
            let new = i.saturating_add(1);
            self.focused[self.curr_workspace] = Some(new);
            self.set_focus(new);
        }
    }

    fn layout(&mut self) {
        if self.curr_ws().is_empty() {
            return;
        }

        let rects = self.window_rects();

        for (i, r) in rects.iter().enumerate() {
            let w = self.curr_ws()[i];
            if self.window_is_dock(w)
                && let Some(spa) = self.strut_partial_atom
            {
                if let Some(strut) = self.get_strut_partial(w, spa) {
                    self.struts.insert(
                        w,
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
            }
            self.x11.move_window(w, r.x, r.y).unwrap();
            self.x11.resize_window(w, r.w as u32, r.h as u32).unwrap();
        }
    }

    fn set_focus(&mut self, index: usize) {
        let w = self.curr_ws()[index];
        if let Some(prev) = self.last_focused {
            self.set_window_border_pixel(prev, self.inactive_border_color);
        }

        self.set_window_border_pixel(w, self.active_border_color);

        let _ = self.x11.raise_window(w);
        let _ = self.x11.focus_window(w);

        self.last_focused = Some(w);
    }
}

fn main() {
    let display_name = std::env::var("DISPLAY").unwrap();
    Nwm::create(&display_name).unwrap().run();
}
