use std::collections::HashSet;

use log::warn;

pub type WindowId = u64;

pub struct X11 {
    display: *mut x11::xlib::Display,
    root: u64,
    event: x11::xlib::XEvent,
    mouse_x: i32,
    mouse_y: i32,
    windows: HashSet<WindowId>,
}

#[derive(Debug)]
pub enum InitError {
    DisplayNotFound(String),
    InvalidDisplayName(std::ffi::NulError),
}

impl From<std::ffi::NulError> for InitError {
    fn from(value: std::ffi::NulError) -> Self {
        Self::InvalidDisplayName(value)
    }
}

impl X11 {
    pub fn init(display_name: &str) -> Result<Self, InitError> {
        let display_string = std::ffi::CString::new(display_name)?;
        let display = unsafe { x11::xlib::XOpenDisplay(display_string.as_ptr()) };

        if display.is_null() {
            return Err(InitError::DisplayNotFound(display_name.to_owned()));
        }

        unsafe {
            x11::xlib::XSelectInput(
                display,
                x11::xlib::XDefaultRootWindow(display),
                x11::xlib::SubstructureRedirectMask
                    | x11::xlib::SubstructureNotifyMask
                    | x11::xlib::KeyPressMask
                    | x11::xlib::PointerMotionMask,
            );
        };
        Ok(Self {
            display,
            root: unsafe { x11::xlib::XDefaultRootWindow(display) },
            event: unsafe { std::mem::zeroed() },
            mouse_x: 0,
            mouse_y: 0,
            windows: HashSet::new(),
        })
    }

    pub fn move_window(&mut self, id: WindowId, x: i32, y: i32) {
        if self.windows.contains(&id) {
            unsafe { x11::xlib::XMoveWindow(self.display, id, x, y) };
            return;
        }
        warn!("Tried to move window-{id} which does not exist");
    }

    pub fn resize_window(&mut self, id: WindowId, w: u32, h: u32) {
        if self.windows.contains(&id) {
            unsafe { x11::xlib::XResizeWindow(self.display, id, w, h) };
            return;
        }
        warn!("Tried to resize window-{id} which does not exist");
    }

    pub fn close_window(&mut self, id: WindowId) {
        if !self.windows.contains(&id) {
            warn!("Tried to close window-{id} which does not exist");
            return;
        }
        unsafe {
            let wm_prot =
                x11::xlib::XInternAtom(self.display, c"WM_PROTOCOLS".as_ptr(), x11::xlib::False);

            let wm_del = x11::xlib::XInternAtom(
                self.display,
                c"WM_DELETE_WINDOW".as_ptr(),
                x11::xlib::False,
            );

            let mut prots: *mut x11::xlib::Atom = std::ptr::null_mut();
            let mut count: i32 = 0;

            if x11::xlib::XGetWMProtocols(self.display, id, &mut prots, &mut count) != 0 {
                let supported = std::slice::from_raw_parts(prots, count as usize)
                    .iter()
                    .any(|&x| x == wm_del);
                x11::xlib::XFree(prots as *mut _);

                if supported {
                    let mut event: x11::xlib::XEvent = std::mem::zeroed();
                    event.client_message.type_ = x11::xlib::ClientMessage;
                    event.client_message.window = id;
                    event.client_message.message_type = wm_prot;
                    event.client_message.format = 32;
                    event.client_message.data.set_long(0, wm_del as i64);
                    event
                        .client_message
                        .data
                        .set_long(1, x11::xlib::CurrentTime as i64);

                    x11::xlib::XSendEvent(
                        self.display,
                        id,
                        x11::xlib::False,
                        x11::xlib::NoEventMask,
                        &mut event,
                    );
                    self.windows.remove(&id);
                    return;
                }
            }
            // fallback for bad clients
            x11::xlib::XKillClient(self.display, id);
            self.windows.remove(&id);
        }
    }

    pub fn grab_key(&mut self, mask: &[MaskKey], key: Key) {
        let mut x11_mask = 0;
        for m in mask {
            x11_mask |= m.to_x11_mask();
        }
        let masks = [
            x11_mask,
            x11_mask | x11::xlib::LockMask,
            x11_mask | x11::xlib::Mod2Mask,
            x11_mask | x11::xlib::LockMask | x11::xlib::Mod2Mask,
        ];

        for m in masks {
            unsafe {
                x11::xlib::XGrabKey(
                    self.display,
                    self.keysym_to_keycode(key.to_x11_keysym()) as i32,
                    m,
                    self.root,
                    1,
                    x11::xlib::GrabModeAsync,
                    x11::xlib::GrabModeAsync,
                );
            }
        }
    }

    pub fn grab_pointer(&mut self) {
        unsafe {
            x11::xlib::XGrabPointer(
                self.display,
                x11::xlib::XDefaultRootWindow(self.display),
                x11::xlib::True,
                (x11::xlib::PointerMotionMask | x11::xlib::EnterWindowMask) as u32,
                x11::xlib::GrabModeAsync,
                x11::xlib::GrabModeAsync,
                0,
                0,
                x11::xlib::CurrentTime,
            );
        }
    }

    pub fn get_mouse_pos(&self) -> (i32, i32) {
        (self.mouse_x, self.mouse_y)
    }

    pub fn unmap_window(&self, id: WindowId) {
        if self.windows.contains(&id) {
            unsafe { x11::xlib::XUnmapWindow(self.display, id) };
            return;
        }
        warn!("Tried to hide window-{id} which does not exist");
    }

    pub fn map_window(&self, id: WindowId) {
        if self.windows.contains(&id) {
            unsafe { x11::xlib::XMapWindow(self.display, id) };
            return;
        }
        warn!("Tried to show window-{id} which does not exist");
    }

    pub fn raise_window(&self, id: WindowId) {
        if self.windows.contains(&id) {
            unsafe { x11::xlib::XMapRaised(self.display, id) };
            return;
        }
        warn!("Tried to raise to top window-{id} which does not exist");
    }

    pub fn focus_window(&self, id: WindowId) {
        if self.windows.contains(&id) {
            unsafe {
                x11::xlib::XSetInputFocus(
                    self.display,
                    id,
                    x11::xlib::RevertToPointerRoot,
                    x11::xlib::CurrentTime,
                );
            }
            return;
        }
        warn!("Tried to focus window-{id} which does not exist");
    }

    pub fn screen_size(&self) -> (i32, i32) {
        unsafe {
            let mut num: i32 = 0;
            let screens = x11::xinerama::XineramaQueryScreens(self.display, &mut num);

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

    pub fn next_event(&mut self) -> Event {
        unsafe { x11::xlib::XNextEvent(self.display, &mut self.event) };

        match self.event.get_type() {
            x11::xlib::MapRequest => {
                self.windows
                    .insert(unsafe { self.event.map_request.window });
                Event::MapRequest(unsafe { self.event.map_request })
            }
            x11::xlib::UnmapNotify => {
                self.windows.remove(&unsafe { self.event.unmap }.window);
                Event::UnmapNotification(unsafe { self.event.unmap })
            }
            x11::xlib::KeyPress => Event::KeyPress(unsafe { self.event.key }),
            x11::xlib::KeyRelease => Event::KeyRelease(unsafe { self.event.key }),
            x11::xlib::MotionNotify => {
                self.mouse_x = unsafe { self.event.motion }.x_root;
                self.mouse_y = unsafe { self.event.motion }.y_root;
                Event::Motion(unsafe { self.event.motion })
            }
            x11::xlib::MappingNotify => {
                unsafe { x11::xlib::XRefreshKeyboardMapping(&mut self.event.mapping) };
                Event::MappingNotify(unsafe { self.event.mapping })
            }
            x11::xlib::CreateNotify => Event::CreateNotify(unsafe { self.event.create_window }),
            x11::xlib::MapNotify => Event::MapNotify(unsafe { self.event.map }),
            x11::xlib::DestroyNotify => Event::DestroyNotify(unsafe { self.event.destroy_window }),
            x11::xlib::ConfigureNotify => Event::ConfigureNotify(unsafe { self.event.configure }),
            x11::xlib::ConfigureRequest => {
                Event::ConfigureRequest(unsafe { self.event.configure_request })
            }
            x11::xlib::ClientMessage => Event::ClientMessage(unsafe { self.event.client_message }),
            x11::xlib::EnterNotify => Event::EnterNotification(unsafe { self.event.visibility }),
            e => todo!("{}", e),
        }
    }

    pub fn keysym_to_keycode(&self, sym: u32) -> u32 {
        unsafe { x11::xlib::XKeysymToKeycode(self.display, sym as u64) as u32 }
    }

    pub fn key_to_keycode<T>(&self, key: T) -> u32
    where
        T: Into<KeySym>,
    {
        unsafe { x11::xlib::XKeysymToKeycode(self.display, key.into().0 as u64) as u32 }
    }
}

pub struct KeySym(pub u32);

impl Drop for X11 {
    fn drop(&mut self) {
        unsafe {
            x11::xlib::XUngrabKey(
                self.display,
                x11::xlib::AnyKey,
                x11::xlib::AnyModifier,
                x11::xlib::XDefaultRootWindow(self.display),
            );
            x11::xlib::XUngrabPointer(self.display, x11::xlib::CurrentTime);
            x11::xlib::XCloseDisplay(self.display);
        }
    }
}

#[derive(Debug)]
pub enum Event {
    MapRequest(x11::xlib::XMapRequestEvent),
    UnmapNotification(x11::xlib::XUnmapEvent),
    KeyPress(x11::xlib::XKeyEvent),
    KeyRelease(x11::xlib::XKeyEvent),
    Motion(x11::xlib::XMotionEvent),
    MappingNotify(x11::xlib::XMappingEvent),
    CreateNotify(x11::xlib::XCreateWindowEvent),
    MapNotify(x11::xlib::XMapEvent),
    DestroyNotify(x11::xlib::XDestroyWindowEvent),
    ConfigureNotify(x11::xlib::XConfigureEvent),
    ConfigureRequest(x11::xlib::XConfigureRequestEvent),
    ClientMessage(x11::xlib::XClientMessageEvent),
    EnterNotification(x11::xlib::XVisibilityEvent),
}

pub enum Key {
    W,
    H,
    L,
    Return,
    Backspace,
    Space,
    One,
    Two,
}

impl Key {
    fn to_x11_keysym(&self) -> u32 {
        match self {
            Key::W => 'w' as u32,
            Key::H => 'h' as u32,
            Key::L => 'l' as u32,
            Key::One => '1' as u32,
            Key::Two => '2' as u32,
            Key::Space => x11::keysym::XK_space,
            Key::Return => x11::keysym::XK_Return,
            Key::Backspace => x11::keysym::XK_BackSpace,
        }
    }
}

pub enum MaskKey {
    Shift,
    Control,
    Super,
    Alt,
}

impl MaskKey {
    fn to_x11_mask(&self) -> u32 {
        match self {
            Self::Alt => x11::xlib::Mod1Mask,
            Self::Super => x11::xlib::Mod4Mask,
            Self::Shift => x11::xlib::ShiftMask,
            Self::Control => x11::xlib::ControlMask,
        }
    }
}
