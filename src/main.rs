use std::slice;

use log::{error, warn, debug, trace, info};

use x11::{xinerama, xlib};


struct Nwm {
    display: *mut xlib::Display,
    windows: Vec<WindowId>,
    focused: Option<usize>,
    gap: u8,
    running: bool,
    conf: Config
}

const MOD_SHIFT: u32 = xlib::ShiftMask;

struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Copy)]
enum MasterKey {
    Super,
    Alt,
    Shift
}

impl Into<u32> for MasterKey {
    fn into(self) -> u32 {
        match self {
            Self::Super => xlib::Mod4Mask,
            Self::Alt => xlib::Mod1Mask,
            Self::Shift => xlib::ShiftMask,
        }
    }
}


#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Config {
    master_key: MasterKey,
}


impl Config {
    pub fn get_master_key(&self) -> u32 {
        self.master_key.into()
    }
}

impl std::default::Default for Config {
    fn default() -> Self {
        Self {
            master_key: MasterKey::Super
        }
    }
}


pub type WindowId = u64;

impl Nwm {
    pub fn create(display_name: &str) -> Option<Self> {
        let s = std::ffi::CString::new(display_name).unwrap();
        let display: *mut xlib::Display =
            unsafe { xlib::XOpenDisplay(s.as_ptr()) };

        if display.is_null() {
            error!("Display {} is not found", display_name);
            return None;
        }

        info!("Succesfully initialized display {} ", display_name);

        unsafe {
            xlib::XSelectInput(
                display,
                xlib::XDefaultRootWindow(display),
                xlib::SubstructureRedirectMask | xlib::SubstructureNotifyMask | xlib::KeyPressMask | xlib::PointerMotionMask,
            );
        };

        let mut conf = Config::default();

        let dirs = platform_dirs::AppDirs::new(Some("nwm"), false).unwrap();
        std::fs::create_dir(&dirs.config_dir);
        let mut conf_dir = dirs.config_dir.clone();
        let mut run_dir = dirs.config_dir.clone();
        conf_dir.push("config.toml");
        run_dir.push("run.sh");

        if conf_dir.exists() {
            let content = std::fs::read_to_string(conf_dir).unwrap();
            conf = toml::de::from_str(&content).unwrap();
        } else {
            std::fs::write(conf_dir, toml::ser::to_string_pretty(&conf).unwrap()).unwrap();
        }

        if run_dir.exists() {
            std::process::Command::new("sh").arg(run_dir).spawn().unwrap();
        }

        info!("Everything went well in initialization :DD");

        Some(
            Self {
                display,
                windows: vec![],
                focused: None,
                gap: 8,
                running: true,
                conf
            }
        )
    }

    fn apply_focus(&self) {
        if let Some(i) = self.focused {
            let w = self.windows[i];
            unsafe {
                xlib::XRaiseWindow(self.display, w);
                xlib::XSetInputFocus(
                    self.display,
                    w,
                    xlib::RevertToPointerRoot,
                    xlib::CurrentTime,
                );
            }
            info!("Applied focus to window {w}");
        }
    }

    fn grab_key(&mut self, key_code: u32, modifiers: u32) {
        unsafe {
            xlib::XGrabKey(self.display, key_code as i32, modifiers, xlib::XDefaultRootWindow(self.display), 1, xlib::GrabModeAsync, xlib::GrabModeAsync);
        }

    }

    pub fn run(mut self) {
        use std::mem::zeroed;
        let mut event: xlib::XEvent = unsafe {zeroed()};
        let enter_code = self.keysym_to_keycode(x11::keysym::XK_Return);
        let h_code = self.keysym_to_keycode(x11::keysym::XK_H);
        let l_code = self.keysym_to_keycode(x11::keysym::XK_L);
        let space_code = self.keysym_to_keycode(x11::keysym::XK_space);
        let w_code = self.keysym_to_keycode(x11::keysym::XK_W);
        let q_code = self.keysym_to_keycode(x11::keysym::XK_Q);

        // launchers
        self.grab_key(enter_code, self.conf.get_master_key());
        self.grab_key(space_code, self.conf.get_master_key());

        // close window
        self.grab_key(w_code, self.conf.get_master_key());

        // close wm
        self.grab_key(q_code, self.conf.get_master_key() | MOD_SHIFT);

        // navigation
        self.grab_key(h_code, self.conf.get_master_key());
        self.grab_key(l_code, self.conf.get_master_key());

        // motion
        self.grab_key(h_code, self.conf.get_master_key() | MOD_SHIFT);
        self.grab_key(l_code, self.conf.get_master_key() | MOD_SHIFT);

        info!("Keybindings were setup");

        while self.running {
            unsafe { xlib::XNextEvent(self.display, &mut event); }

            match event.get_type() {
                xlib::MapRequest => self.add_window(event.into()),
                xlib::UnmapNotify => self.remove_window(event.into()),
                xlib::KeyPress => {
                    let key_event = unsafe { event.key };
                    if key_event.state & self.conf.get_master_key() != 0 && key_event.state & MOD_SHIFT == 0 {
                        match key_event.keycode {
                            x if x == enter_code => {
                                std::process::Command::new("alacritty")
                                    .spawn().unwrap();
                            },
                            x if x == space_code => {
                                std::process::Command::new("dmenu_run")
                                    .spawn().unwrap();
                            }
                            x if x == h_code => {
                                self.focus_left();
                            },
                            x if x == l_code => {
                                self.focus_right();
                            },
                            x if x == w_code => {
                                if let Some(w) = self.focused {
                                    unsafe {
                                        xlib::XUnmapWindow(self.display, self.windows[w]);
                                    }
                                    self.windows.remove(w);
                                }
                            },
                            _ => {}
                        }
                    } else if key_event.state & self.conf.get_master_key() != 0 && key_event.state & MOD_SHIFT != 0 {
                        match key_event.keycode {
                            x if x == h_code => {
                                self.swap_left();
                            },
                            x if x == l_code => {
                                self.swap_right();
                            },
                            x if x == q_code => {
                                info!("Exiting nwm, byee!");
                                self.running = false;
                            }
                            _ => {}
                        }
                    }
                }
                xlib::KeyRelease => {},
                xlib::MotionNotify => {
                    let motion_event = unsafe {event.motion};
                    let rects = self.window_rects();
                    for (i, r) in rects.iter().enumerate() {
                        if motion_event.x > r.x && motion_event.x < r.x + r.w {
                            self.focused = Some(i);
                            self.apply_focus();
                        }
                    }
                }
                xlib::MappingNotify => {
                    let mut e = unsafe {event.mapping};
                    unsafe { xlib::XRefreshKeyboardMapping(&mut e); }
                }
                xlib::CreateNotify | xlib::MapNotify | xlib::DestroyNotify | xlib::ConfigureNotify => {
                }
                xlib::ConfigureRequest => {
                    self.layout();
                }
                _ => {
                    warn!("Unknown event: {:#?}", event);
                }
            }
        }
        unsafe {
            xlib::XUngrabKey(self.display, xlib::AnyKey, xlib::AnyModifier, xlib::XDefaultRootWindow(self.display));
            xlib::XCloseDisplay(self.display);
        }

    }

    fn window_rects(&self) -> Vec<Rect> {
        let mut rs = vec![];
        let (sw, sh) = self.screen_size();

        let n = self.windows.len() as i32;
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
        unsafe {xlib::XMapWindow(self.display, event.window);}
        self.windows.push(event.window);
        self.focused = Some(self.windows.len() - 1);
        self.layout();
        unsafe {xlib::XMapRaised(self.display, event.window);}
        unsafe {xlib::XSetInputFocus(self.display, event.window, xlib::RevertToPointerRoot, xlib::CurrentTime);}
    }

    fn remove_window(&mut self, event: xlib::XUnmapEvent) {
        if let Some(pos) = self.windows.iter().position(|&w| w == event.window) {
            self.windows.remove(pos);
            if let Some(f) = self.focused {
                if f >= self.windows.len() {
                    self.focused = self.windows.len().checked_sub(1);
                }
            }
        }
        self.layout();
    }

    fn screen_size(&self) -> (i16, i16) {
        unsafe {
            let mut num: i32 = 0;
            let screen_pointers = xinerama::XineramaQueryScreens(self.display, &mut num);
            let screens = slice::from_raw_parts(screen_pointers, num as usize).to_vec();
            (screens[0].width, screens[0].height)
        }
    }
    fn swap_left(&mut self) {
        if let Some(i) = self.focused {
            if i > 0 {
                self.windows.swap(i, i - 1);
                self.focused = Some(i - 1);
                self.layout();
            }
        }
    }

    fn swap_right(&mut self) {
        if let Some(i) = self.focused {
            if i + 1 < self.windows.len() {
                self.windows.swap(i, i + 1);
                self.focused = Some(i + 1);
                self.layout();
            }
        }
    }

    fn focus_left(&mut self) {
        self.focused = self.focused.map(|x| if x == 0 {x} else {x - 1});
        self.apply_focus();
    }
    fn focus_right(&mut self) {
        self.focused = self.focused.map(|x| if self.windows.len() - 1 == x {x} else {x + 1});
        self.apply_focus();
    }

    fn layout(&mut self) {
        if self.windows.is_empty() {
            return;
        }

        let rects = self.window_rects();

        for (i, r) in rects.iter().enumerate() {
            let w = self.windows[i];
            self.move_window(w, r.x, r.y);
            self.resize_window(w, r.w as u32, r.h as u32);
        }
    }

    fn move_window(&self, w: u64, x: i32, y: i32) {
        info!("Moved window {w} to {x}:{y}");
        unsafe {
            xlib::XMoveWindow(self.display, w, x, y);
        }
    }
    fn resize_window(&self, w: u64, width: u32, height: u32) {
        info!("Resized window {w} to {width}x{height}");
        unsafe {xlib::XResizeWindow(self.display, w, width, height);}
    }

    fn keysym_to_keycode(&self, sym: u32) -> u32 {
        unsafe {
            xlib::XKeysymToKeycode(self.display, sym as u64) as u32
        }
    }
}

fn main() {
    env_logger::init();
    let display_name = std::env::var("DISPLAY").unwrap();
    Nwm::create(&display_name).unwrap().run();
}

