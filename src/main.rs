mod config;
use std::str::FromStr;

use log::{debug, error, info, trace, warn};

use x11::{xinerama, xlib};

struct Nwm {
    display: *mut xlib::Display,
    workspaces: [Vec<WindowId>; 10],
    curr_workspace: usize,
    focused: [Option<usize>; 10],
    running: bool,
    last_x: i32,
    last_y: i32,
    gap: u8,
    master_key: MasterKey,
    binds: Vec<Bind>,
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
    fn grab(&self, display: *mut xlib::Display) {
        let base_mask = keycombo_mask(&self.bind);
        let keycode = keysym_to_keycode(key_to_keysym(self.bind.key), display);

        let masks = [
            base_mask,
            base_mask | xlib::LockMask,
            base_mask | xlib::Mod2Mask,
            base_mask | xlib::LockMask | xlib::Mod2Mask,
        ];

        for m in masks {
            unsafe {
                xlib::XGrabKey(
                    display,
                    keycode as i32,
                    m,
                    xlib::XDefaultRootWindow(display),
                    1,
                    xlib::GrabModeAsync,
                    xlib::GrabModeAsync,
                );
            }
        }
    }

    fn try_do(&self, nwm: &mut Nwm, ev: &xlib::XKeyEvent) {
        let want_keycode = keysym_to_keycode(key_to_keysym(self.bind.key), nwm.display);

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

const MOD_SHIFT: u32 = xlib::ShiftMask;

struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy)]
enum MasterKey {
    Super,
    Shift,
    Control,
    Alt,
}

impl Into<u32> for MasterKey {
    fn into(self) -> u32 {
        match self {
            Self::Super => xlib::Mod4Mask,
            Self::Alt => xlib::Mod1Mask,
            Self::Shift => xlib::ShiftMask,
            Self::Control => xlib::ControlMask,
        }
    }
}

impl FromStr for MasterKey {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Super" => Ok(MasterKey::Super),
            "Alt" => Ok(MasterKey::Alt),
            "Shift" => Ok(MasterKey::Shift),
            "Control" => Ok(MasterKey::Control),
            _ => Err(()),
        }
    }
}

pub type WindowId = u64;

fn action_to_fn(action: config::Action) -> fn(&mut Nwm) {
    match action {
        config::Action::FocusLeft => Nwm::focus_left,
        config::Action::FocusRight => Nwm::focus_right,
        config::Action::Launcher => Nwm::launcher,
        config::Action::Terminal => Nwm::terminal,
        config::Action::CloseWindow => Nwm::close_focused,
        config::Action::NextWs => Nwm::focus_next_ws,
        config::Action::PrevWs => Nwm::focus_prev_ws,
    }
}

fn mod_to_mask(m: config::SpecialKey) -> u32 {
    match m {
        config::SpecialKey::Alt => xlib::Mod1Mask,
        config::SpecialKey::Super => xlib::Mod4Mask,
        config::SpecialKey::Control => xlib::ControlMask,
        config::SpecialKey::Shift => xlib::ShiftMask,
        _ => 0,
    }
}

fn key_to_keysym(c: char) -> u32 {
    match c {
        ' ' => x11::keysym::XK_space,
        '\n' => x11::keysym::XK_Return,
        '\t' => x11::keysym::XK_Tab,
        _ => c as u32,
    }
}

impl Nwm {
    pub fn create(display_name: &str) -> Option<Self> {
        let s = std::ffi::CString::new(display_name).unwrap();
        let display: *mut xlib::Display = unsafe { xlib::XOpenDisplay(s.as_ptr()) };

        if display.is_null() {
            error!("Display {} is not found", display_name);
            return None;
        }

        info!("Succesfully initialized display {} ", display_name);

        unsafe {
            xlib::XSelectInput(
                display,
                xlib::XDefaultRootWindow(display),
                xlib::SubstructureRedirectMask
                    | xlib::SubstructureNotifyMask
                    | xlib::KeyPressMask
                    | xlib::PointerMotionMask,
            );
        };

        let mut conf = vec![];

        let dirs = platform_dirs::AppDirs::new(Some("nwm"), false).unwrap();
        _ = std::fs::create_dir(&dirs.config_dir);
        let mut conf_dir = dirs.config_dir.clone();
        let mut run_dir = dirs.config_dir.clone();
        conf_dir.push("config.nwc");
        run_dir.push("run.sh");

        let mut gap = 0;
        let mut master_key = MasterKey::Super;
        let mut binds = vec![];

        if conf_dir.exists() {
            let content = std::fs::read_to_string(conf_dir).unwrap();
            conf = config::parse(content).unwrap();
            for s in conf {
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
                        _ => warn!("Invalid Set statement"),
                    },

                    config::Statement::Do { action, on } => {
                        let bind = Bind {
                            action: action_to_fn(action),
                            bind: on,
                        };
                        bind.grab(display);
                        binds.push(bind);
                    }
                }
            }
        } else {
            warn!("TODO: Serialize config");
        }

        if run_dir.exists() {
            std::process::Command::new("sh").arg(run_dir).spawn().ok();
        }

        info!("Everything went well in initialization :DD");

        Some(Self {
            display,
            workspaces: Default::default(),
            curr_workspace: 0,
            focused: Default::default(),
            gap,
            master_key,
            running: true,
            last_x: 0,
            last_y: 0,
            binds,
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

    fn is_focused(&self) -> bool {
        self.focused().is_some()
    }

    fn apply_focus(&mut self) {
        if let Some(i) = self.focused() {
            if self.curr_ws().len() <= i {
                return;
            }
            let w = self.workspaces[self.curr_workspace][i];
            unsafe {
                xlib::XRaiseWindow(self.display, w);
                xlib::XSetInputFocus(self.display, w, xlib::RevertToParent, xlib::CurrentTime);
            }
            info!("Applied focus to window {w}");
        }
    }

    fn grab_key(&mut self, key_code: u32, modifiers: u32) {
        unsafe {
            xlib::XGrabKey(
                self.display,
                key_code as i32,
                modifiers,
                xlib::XDefaultRootWindow(self.display),
                1,
                xlib::GrabModeAsync,
                xlib::GrabModeAsync,
            );
        }
    }

    fn grab_pointer(&mut self) {
        unsafe {
            xlib::XGrabPointer(
                self.display,
                xlib::XDefaultRootWindow(self.display),
                xlib::True,
                (xlib::PointerMotionMask | xlib::EnterWindowMask) as u32,
                xlib::GrabModeAsync,
                xlib::GrabModeAsync,
                0,
                0,
                xlib::CurrentTime,
            );
        }
    }

    fn unmap_window(&mut self, window_index: usize) {
        if window_index < self.curr_ws().len() {
            unsafe {
                xlib::XUnmapWindow(self.display, self.curr_ws()[window_index]);
            }
        }
    }

    fn forget_window(&mut self, window_index: usize) {
        if window_index < self.workspaces[self.curr_workspace].len() {
            self.unmap_window(window_index);
            self.curr_ws_mut().remove(window_index);
        }
    }

    fn close_focused(&mut self) {
        if let Some(w) = self.focused() {
            self.close_window(w);
        }
    }

    fn close_window(&mut self, window_index: usize) {
        if self.curr_ws().len() <= window_index {
            warn!("Tried to close window that does not exist");
            return;
        }
        unsafe {
            let wm_prot = xlib::XInternAtom(
                self.display,
                c"WM_PROTOCOLS".as_ptr(),
                xlib::False,
            );

            let wm_del = xlib::XInternAtom(
                self.display,
                c"WM_DELETE_WINDOW".as_ptr(),
                xlib::False,
            );

            let mut prots: *mut xlib::Atom = std::ptr::null_mut();
            let mut count: i32 = 0;

            if xlib::XGetWMProtocols(
                self.display,
                self.curr_ws()[window_index],
                &mut prots,
                &mut count,
            ) != 0
            {
                let supported = std::slice::from_raw_parts(prots, count as usize)
                    .iter()
                    .any(|&x| x == wm_del);
                xlib::XFree(prots as *mut _);

                if supported {
                    let mut event: xlib::XEvent = std::mem::zeroed();
                    event.client_message.type_ = xlib::ClientMessage;
                    event.client_message.window = self.curr_ws()[window_index];
                    event.client_message.message_type = wm_prot;
                    event.client_message.format = 32;
                    event.client_message.data.set_long(0, wm_del as i64);
                    event
                        .client_message
                        .data
                        .set_long(1, xlib::CurrentTime as i64);

                    xlib::XSendEvent(
                        self.display,
                        self.curr_ws()[window_index],
                        xlib::False,
                        xlib::NoEventMask,
                        &mut event,
                    );
                    self.curr_ws_mut().remove(window_index);
                    return;
                }
            }
            // fallback for bad clients
            xlib::XKillClient(self.display, self.curr_ws()[window_index]);
            self.curr_ws_mut().remove(window_index);
        }
    }

    fn curr_ws_mut(&mut self) -> &mut Vec<WindowId> {
        &mut self.workspaces[self.curr_workspace]
    }
    fn curr_ws(&self) -> &Vec<WindowId> {
        &self.workspaces[self.curr_workspace]
    }

    fn refresh_mappings(&mut self, mut e: x11::xlib::XMappingEvent) {
        unsafe {
            xlib::XRefreshKeyboardMapping(&mut e);
        }
    }

    pub fn run(mut self) {
        use std::mem::zeroed;

        self.grab_pointer();

        info!("Keybindings were setup");

        let mut event: xlib::XEvent = unsafe { zeroed() };
        while self.running {
            unsafe {
                xlib::XNextEvent(self.display, &mut event);
            }

            match event.get_type() {
                xlib::MapRequest => self.add_window(unsafe { event.map_request }),
                xlib::UnmapNotify => self.remove_window(unsafe { event.unmap }),
                xlib::KeyPress => {
                    let key_event = unsafe { event.key };
                    for b in self.binds.clone() {
                        b.try_do(&mut self, &key_event);
                    }
                }
                xlib::KeyRelease => {}
                xlib::MotionNotify => {
                    let motion_event = unsafe { event.motion };
                    if self.last_x != motion_event.x_root && self.last_y != motion_event.y_root {
                        let rects = self.window_rects();
                        for (i, r) in rects.iter().enumerate() {
                            if motion_event.x_root > r.x && motion_event.x_root < r.x + r.w {
                                self.focused[self.curr_workspace] = Some(i);
                                self.apply_focus();
                            }
                        }
                        self.last_x = motion_event.x_root;
                        self.last_y = motion_event.y_root;
                    }
                }
                xlib::MappingNotify => {
                    self.refresh_mappings(unsafe { event.mapping });
                }
                xlib::CreateNotify
                | xlib::MapNotify
                | xlib::DestroyNotify
                | xlib::ConfigureNotify => {}
                xlib::ConfigureRequest => self.layout(),
                _ => {
                    warn!("Unknown event: {:#?}", event);
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

        // Unmap old workspace windows
        for &w in &self.workspaces[old_ws] {
            unsafe {
                xlib::XUnmapWindow(self.display, w);
            }
        }

        self.curr_workspace = new_ws;

        for &w in &self.workspaces[new_ws] {
            unsafe {
                xlib::XMapWindow(self.display, w);
            }
        }

        self.layout();
        self.focus_on_pointer();
    }
    fn window_rects(&self) -> Vec<Rect> {
        let mut rs = vec![];
        let (sw, sh) = self.screen_size();

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
        unsafe {
            xlib::XMapWindow(self.display, event.window);
        }
        self.curr_ws_mut().push(event.window);
        self.focused[self.curr_workspace] = Some(self.curr_ws().len() - 1);
        self.layout();
        unsafe {
            xlib::XMapRaised(self.display, event.window);
        }
        unsafe {
            xlib::XSetInputFocus(
                self.display,
                event.window,
                xlib::RevertToPointerRoot,
                xlib::CurrentTime,
            );
        }
    }

    fn map_window(&mut self, window_index: usize) {
        unsafe {
            xlib::XMapWindow(self.display, self.curr_ws()[window_index]);
        }
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

    fn screen_size(&self) -> (i32, i32) {
        unsafe {
            let mut num: i32 = 0;
            let screens = xinerama::XineramaQueryScreens(self.display, &mut num);

            let mut max_x = 0;
            let mut max_y = 0;

            for i in 0..num {
                let s = *screens.add(i as usize);
                max_x = max_x.max(s.x_org + s.width);
                max_y = max_y.max(s.y_org + s.height);
            }

            (max_x as i32, max_y as i32)
        }
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
        std::process::Command::new("dmenu_run").spawn().unwrap();
    }

    fn terminal(&mut self) {
        std::process::Command::new("alacritty").spawn().unwrap();
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
            self.move_window(w, r.x, r.y);
            self.resize_window(w, r.w as u32, r.h as u32);
        }
    }

    fn move_window(&self, w: u64, x: i32, y: i32) {
        trace!("Moved window {w} to {x}:{y}");
        unsafe {
            xlib::XMoveWindow(self.display, w, x, y);
        }
    }
    fn resize_window(&self, w: u64, width: u32, height: u32) {
        trace!("Resized window {w} to {width}x{height}");
        unsafe {
            xlib::XResizeWindow(self.display, w, width, height);
        }
    }


}
fn keysym_to_keycode(sym: u32, display: *mut xlib::Display) -> u32 {
    unsafe { xlib::XKeysymToKeycode(display, sym as u64) as u32 }
}

impl Drop for Nwm {
    fn drop(&mut self) {
        unsafe {
            xlib::XUngrabKey(
                self.display,
                xlib::AnyKey,
                xlib::AnyModifier,
                xlib::XDefaultRootWindow(self.display),
            );
            xlib::XUngrabPointer(self.display, xlib::CurrentTime);
            xlib::XCloseDisplay(self.display);
        }
    }
}

fn main() {
    env_logger::init();
    let display_name = std::env::var("DISPLAY").unwrap();
    Nwm::create(&display_name).unwrap().run();
}
