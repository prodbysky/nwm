mod better_x11;
mod config;

use log::{info, warn};

use x11::xlib;

struct Nwm {
    x11: better_x11::X11,
    workspaces: [Vec<WindowId>; 10],
    curr_workspace: usize,
    focused: [Option<usize>; 10],
    running: bool,
    last_x: i32,
    last_y: i32,
    gap: u8,
    binds: Vec<Bind>,
    terminal: String,
    launcher: String,
}

const IGNORED_MODS: u32 = xlib::LockMask | xlib::Mod2Mask; // CapsLock + NumLock

#[derive(Debug, Clone)]
struct Bind {
    action: fn(&mut Nwm),
    bind: config::KeyCombo,
}

fn keycombo_mask(kc: &config::KeyCombo) -> u32 {
    let mut mask = 0;
    for m in &kc.prefixes {
        mask |= match m {
            config::SpecialKey::Shift => xlib::ShiftMask,
            config::SpecialKey::Control => xlib::ControlMask,
            config::SpecialKey::Alt => xlib::Mod1Mask,
            config::SpecialKey::Super => xlib::Mod4Mask,
            _ => 0,
        };
    }
    mask
}

impl Bind {
    fn try_do(&self, nwm: &mut Nwm, ev: &xlib::XKeyEvent) {
        let want_keycode = nwm
            .x11
            .keysym_to_keycode(key_to_keysym(self.bind.key.clone()));

        if ev.keycode as u32 != want_keycode {
            return;
        }

        let want_mask = keycombo_mask(&self.bind);
        let actual_mask = ev.state & !IGNORED_MODS;

        if actual_mask != want_mask {
            return;
        }

        (self.action)(nwm);
    }
}

struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

use serde::{Deserialize, Serialize};

use crate::better_x11::Event;

#[derive(Serialize, Deserialize, Clone, Copy)]
enum MasterKey {
    Super,
    Shift,
    Control,
    Alt,
}

pub type WindowId = u64;

fn action_to_fn(action: config::Action) -> fn(&mut Nwm) {
    match action {
        config::Action::FocusLeft => Nwm::focus_left,
        config::Action::FocusRight => Nwm::focus_right,
        config::Action::MoveLeft => Nwm::swap_left,
        config::Action::MoveRight => Nwm::swap_right,
        config::Action::Launcher => Nwm::launcher,
        config::Action::Terminal => Nwm::terminal,
        config::Action::CloseWindow => Nwm::close_focused,
        config::Action::NextWs => Nwm::focus_next_ws,
        config::Action::PrevWs => Nwm::focus_prev_ws,
    }
}

fn key_to_keysym(key: config::Key) -> u32 {
    match key {
        config::Key::Char(c) => c as u32,
        config::Key::Space => x11::keysym::XK_space,
        config::Key::Return => x11::keysym::XK_Return,
        config::Key::Tab => x11::keysym::XK_Tab,
        config::Key::Escape => x11::keysym::XK_Escape,
    }
}

impl Nwm {
    fn apply_config(
        conf: config::Config,
        ab: &mut better_x11::X11,
    ) -> (u8, MasterKey, Vec<Bind>, String, String) {
        let mut gap = 0;
        let mut master_key = MasterKey::Alt;
        let mut binds = vec![];
        let mut terminal = String::new();
        let mut launcher = String::new();
        for s in conf.0 {
            match s {
                config::Statement::Set { var, value } => match (var, value) {
                    (config::Variable::Gap, config::Value::Num(n)) => {
                        gap = n as u8;
                    }
                    (config::Variable::MasterKey, config::Value::Key(k)) => {
                        master_key = match k {
                            config::SpecialKey::Super => MasterKey::Super,
                            config::SpecialKey::Shift => MasterKey::Shift,
                            config::SpecialKey::Alt => MasterKey::Alt,
                            config::SpecialKey::Control => MasterKey::Control,
                            _ => continue,
                        };
                    }
                    (config::Variable::Terminal, config::Value::String(k)) => {
                        terminal = k;
                    }
                    (config::Variable::Launcher, config::Value::String(k)) => {
                        launcher = k;
                    }
                    _ => warn!("Invalid Set statement"),
                },

                config::Statement::Do { action, mut on } => {
                    on.prefixes.insert(0, master_key.into());

                    let mask: Vec<_> = on
                        .prefixes
                        .iter()
                        .map(|k| match k {
                            config::SpecialKey::Alt => better_x11::MaskKey::Alt,
                            config::SpecialKey::Shift => better_x11::MaskKey::Shift,
                            config::SpecialKey::Control => better_x11::MaskKey::Control,
                            config::SpecialKey::Super => better_x11::MaskKey::Super,
                            config::SpecialKey::Space => unreachable!(),
                        })
                        .collect();

                    ab.keysym_to_keycode(key_to_keysym(on.key));
                    ab.grab_key(&mask, on.key.into());

                    binds.push(Bind {
                        action: action_to_fn(action),
                        bind: on,
                    });
                }
            }
        }
        (gap, master_key, binds, terminal, launcher)
    }

    pub fn create(display_name: &str) -> Option<Self> {
        let mut x11_ab = better_x11::X11::init(display_name).unwrap();

        x11_ab.grab_pointer();

        info!("Succesfully initialized display {} ", display_name);

        let dirs = platform_dirs::AppDirs::new(Some("nwm"), false).unwrap();
        _ = std::fs::create_dir(&dirs.config_dir);
        let mut conf_dir = dirs.config_dir.clone();
        let mut run_dir = dirs.config_dir.clone();
        conf_dir.push("config.nwc");
        run_dir.push("run.sh");

        let gap;
        let mut binds = vec![];

        let launcher;
        let terminal;

        let conf;

        if conf_dir.exists() {
            let content = std::fs::read_to_string(conf_dir).unwrap();
            conf = config::Config::parse(content).unwrap();
            (gap, _, binds, terminal, launcher) = Self::apply_config(conf, &mut x11_ab);
        } else {
            conf = config::Config::default();
            _ = std::fs::write(&conf_dir, conf.to_string());
            (gap, _, binds, terminal, launcher) = Self::apply_config(conf, &mut x11_ab);
        }

        if run_dir.exists() {
            std::process::Command::new("sh").arg(run_dir).spawn().ok();
        }

        info!("Everything went well in initialization :DD");
        if launcher.is_empty() {
            warn!("Launcher wasn't set to a program");
        }
        if terminal.is_empty() {
            warn!("Terminal wasn't set to a program");
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
        })
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

    fn apply_focus(&mut self) {
        if let Some(i) = self.focused() {
            if self.curr_ws().len() <= i {
                return;
            }
            let w = self.workspaces[self.curr_workspace][i];
            self.x11.raise_window(w);
            self.x11.focus_window(w);
            info!("Applied focus to window {w}");
        }
    }

    fn close_focused(&mut self) {
        if let Some(w) = self.focused() {
            self.close_window(w);
        }
    }

    fn close_window(&mut self, window_index: usize) {
        if window_index < self.curr_ws().len() {
            self.x11.close_window(self.curr_ws()[window_index]);
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
            let event = self.x11.next_event();

            match event {
                Event::MapRequest(e) => self.add_window(e),
                Event::UnmapNotification(e) => self.remove_window(e),
                Event::KeyPress(e) => {
                    for b in &self.binds.clone() {
                        b.try_do(&mut self, &e);
                    }
                }
                Event::Motion(_) => {
                    let (x, y) = self.x11.get_mouse_pos();
                    if self.last_x != x && self.last_y != y {
                        let rects = self.window_rects();
                        for (i, r) in rects.iter().enumerate() {
                            if x > r.x && x < r.x + r.w {
                                self.focused[self.curr_workspace] = Some(i);
                                self.apply_focus();
                            }
                        }
                        self.last_x = x;
                        self.last_y = y;
                    }
                }
                Event::KeyRelease(_) => {}
                Event::MappingNotify(_) => {}
                Event::ConfigureRequest(_) => self.layout(),
                Event::CreateNotify(_)
                | Event::MapNotify(_)
                | Event::DestroyNotify(_)
                | Event::ConfigureNotify(_) => {}
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
                if self.focused().is_none() {
                    self.focused[self.curr_workspace] = Some(i);
                }
                self.apply_focus();
            }
        }
    }

    fn switch_ws(&mut self, new_ws: usize) {
        if new_ws >= self.workspaces.len() || new_ws == self.curr_workspace {
            return;
        }

        let old_ws = self.curr_workspace;

        for &w in &self.workspaces[old_ws] {
            self.x11.unmap_window(w);
        }

        self.curr_workspace = new_ws;

        for &w in &self.workspaces[new_ws] {
            self.x11.map_window(w);
        }

        self.layout();
        self.focus_on_pointer();
    }

    fn window_rects(&self) -> Vec<Rect> {
        let mut rs = vec![];
        let (sw, sh) = self.x11.screen_size();

        let n = self.curr_ws().len() as i32;
        if n == 0 {
            return rs;
        }

        let gap = self.gap as i32;
        let half_gap = gap / 2;

        let usable_w = sw as i32 - gap * 2;
        let slot_w = usable_w / n;

        for i in 0..n {
            let x = gap + i * slot_w + half_gap;
            let y = gap;

            let w = slot_w - half_gap * 2;
            let h = sh as i32 - gap * 2;

            if w > 0 && h > 0 {
                rs.push(Rect { x, y, w, h });
            }
        }

        rs
    }

    fn add_window(&mut self, event: xlib::XMapRequestEvent) {
        self.x11.map_window(event.window);
        self.curr_ws_mut().push(event.window);
        self.focused[self.curr_workspace] = Some(self.curr_ws().len() - 1);
        self.layout();
        self.x11.raise_window(event.window);
        self.x11.focus_window(event.window);
    }

    fn remove_window(&mut self, event: xlib::XUnmapEvent) {
        if let Some(pos) = self.curr_ws().iter().position(|&w| w == event.window) {
            self.curr_ws_mut().remove(pos);
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
                self.apply_focus();
            }
        }
    }

    fn swap_right(&mut self) {
        if let Some(i) = self.focused() {
            if i + 1 < self.curr_ws().len() {
                self.curr_ws_mut().swap(i, i + 1);
                self.focused[self.curr_workspace] = Some(i + 1);
                self.layout();
                self.apply_focus();
            }
        }
    }

    fn launcher(&mut self) {
        std::process::Command::new(&self.launcher).spawn().unwrap();
    }

    fn terminal(&mut self) {
        std::process::Command::new(&self.terminal).spawn().unwrap();
    }

    fn focus_left(&mut self) {
        self.focused[self.curr_workspace] =
            self.focused[self.curr_workspace].map(|x| if x == 0 { x } else { x - 1 });
        self.apply_focus();
    }
    fn focus_right(&mut self) {
        self.focused[self.curr_workspace] = self.focused[self.curr_workspace].map(|x| {
            if self.curr_ws().len() - 1 == x {
                x
            } else {
                x + 1
            }
        });
        self.apply_focus();
    }

    fn layout(&mut self) {
        if self.curr_ws().is_empty() {
            return;
        }

        let rects = self.window_rects();

        for (i, r) in rects.iter().enumerate() {
            let w = self.curr_ws()[i];
            self.x11.move_window(w, r.x, r.y);
            self.x11.resize_window(w, r.w as u32, r.h as u32);
        }
    }
}

fn main() {
    env_logger::init();
    let display_name = std::env::var("DISPLAY").unwrap();
    Nwm::create(&display_name).unwrap().run();
}
