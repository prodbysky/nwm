use std::collections::HashMap;

use log::{error, warn};

use x11rb::{
    connection::Connection,
    protocol::{
        Event,
        xproto::{
            Atom, ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt, EventMask,
            GrabMode, InputFocus, Keycode, MappingNotifyEvent, ModMask, Screen, StackMode, Time,
        },
    },
    rust_connection::RustConnection,
};

pub type WindowId = u32;

pub struct X11RB {
    pub conn: RustConnection,
    screen: Screen,
    pointer_pos: (i16, i16),
    keymap: HashMap<u32, Keycode>,
}

impl X11RB {
    pub fn init() -> Option<Self> {
        let (conn, screen_number) = x11rb::connect(None)
            .map_err(|e| {
                error!("Failed to open x11 display: {e}");
            })
            .ok()?;
        let screen = &conn.setup().roots[screen_number];
        let root = screen.root;

        let event_mask = EventMask::SUBSTRUCTURE_REDIRECT
            | EventMask::SUBSTRUCTURE_NOTIFY
            | EventMask::KEY_PRESS
            | EventMask::POINTER_MOTION
            | EventMask::PROPERTY_CHANGE
            | EventMask::ENTER_WINDOW
            | EventMask::LEAVE_WINDOW;

        conn.change_window_attributes(
            root,
            &ChangeWindowAttributesAux::new().event_mask(event_mask),
        )
        .map_err(|e| error!("Failed to set window event mask: {e}"))
        .ok()?;

        let mut wm = Self {
            screen: screen.clone(),
            conn,
            pointer_pos: (0, 0),
            keymap: HashMap::new(),
        };
        wm.rebuild_keymap();

        Some(wm)
    }

    pub fn root_window(&self) -> u32 {
        self.screen.root
    }

    pub fn intern_atom(&mut self, name: &[u8]) -> Option<Atom> {
        let a = self
            .conn
            .intern_atom(false, name)
            .map_err(|e| {
                warn!(
                    "Failed to intern atom {}: {e}",
                    str::from_utf8(name).unwrap()
                );
            })
            .ok()?
            .reply()
            .map_err(|e| {
                warn!("Failed to read x11 reply: {e}");
            })
            .ok()?;
        Some(a.atom)
    }

    fn rebuild_keymap(&mut self) -> Option<()> {
        self.keymap.clear();
        let setup = self.conn.setup();
        let min = setup.min_keycode;
        let max = setup.max_keycode;

        let reply = self
            .conn
            .get_keyboard_mapping(min, max - min + 1)
            .map_err(|e| {
                warn!("Failed to rebuild keymap : {e}",);
            })
            .ok()?
            .reply()
            .map_err(|e| {
                warn!("Failed to get reply from get_keyboard_mapping : {e}");
            })
            .ok()?;

        let per = reply.keysyms_per_keycode as usize;

        for (i, syms) in reply.keysyms.chunks(per).enumerate() {
            let keycode = min + i as u8;
            for &keysym in syms {
                self.keymap.insert(keysym, keycode);
            }
        }
        Some(())
    }

    pub fn grab_pointer(&mut self) -> Option<()> {
        self.conn
            .grab_pointer(
                true,
                self.screen.root,
                x11rb::protocol::xproto::EventMask::POINTER_MOTION
                    | x11rb::protocol::xproto::EventMask::ENTER_WINDOW,
                x11rb::protocol::xproto::GrabMode::ASYNC,
                x11rb::protocol::xproto::GrabMode::ASYNC,
                0_u32,
                0_u32,
                x11rb::protocol::xproto::Time::CURRENT_TIME,
            )
            .map_err(|e| {
                warn!("Failed to grab the pointer: {e}");
            })
            .ok()?;
        Some(())
    }

    fn flush(&mut self) -> Option<()> {
        self.conn
            .flush()
            .map_err(|e| {
                warn!("Failed to flush the x11 connection: {e}");
            })
            .ok()
    }

    pub fn next_event(&mut self) -> Option<Event> {
        self.conn.flush().unwrap();
        let e = self
            .conn
            .wait_for_event()
            .map_err(|e| {
                warn!("Failed to get the next x11 event: {e}");
            })
            .ok()?;

        match e {
            Event::MotionNotify(e) => {
                self.pointer_pos.0 = e.root_x;
                self.pointer_pos.1 = e.root_y;
            }
            Event::MappingNotify(e) => {
                // NOTE: I can't actually understand this if stuff breaks look at THIS
                self.update_mapping(e);
            }
            _ => {}
        };

        Some(e)
    }

    pub fn mouse_pos(&self) -> (i16, i16) {
        self.pointer_pos
    }

    pub fn update_mapping(&mut self, _: MappingNotifyEvent) {
        self.rebuild_keymap();
    }

    pub fn screen_size(&self) -> (u16, u16) {
        (self.screen.width_in_pixels, self.screen.height_in_pixels)
    }

    pub fn raise_window(&mut self, id: WindowId) -> Option<()> {
        self.conn
            .map_window(id)
            .map_err(|e| {
                warn!("Failed to map window {id} (while raising it): {e}");
            })
            .ok()?;
        self.conn
            .configure_window(id, &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE))
            .map_err(|e| {
                warn!("Failed to raise window {id} to top: {e}");
            })
            .ok()?;
        Some(())
    }

    pub fn map_window(&mut self, id: WindowId) -> Option<()> {
        self.conn
            .map_window(id)
            .map_err(|e| {
                warn!("Failed to map window {id}: {e}");
            })
            .ok()?;
        Some(())
    }

    pub fn unmap_window(&mut self, id: WindowId) -> Option<()> {
        self.conn
            .unmap_window(id)
            .map_err(|e| {
                warn!("Failed to unmap window {id}: {e}");
            })
            .ok()?;
        Some(())
    }

    pub fn focus_window(&mut self, id: WindowId) -> Option<()> {
        self.conn
            .set_input_focus(InputFocus::PARENT, id, Time::CURRENT_TIME)
            .map_err(|e| {
                warn!("Failed to focus window {id}: {e}");
            })
            .ok()?;
        Some(())
    }

    pub fn close_window(&mut self, id: WindowId) -> Option<()> {
        self.conn
            .destroy_window(id)
            .map_err(|e| {
                warn!("Failed to close window {id}: {e}");
            })
            .ok()?;
        Some(())
    }

    pub fn move_window(&mut self, id: WindowId, x: i16, y: i16) -> Option<()> {
        self.conn
            .configure_window(id, &ConfigureWindowAux::new().x(x as i32).y(y as i32))
            .map_err(|e| {
                warn!("Failed to move window {id}: {e}");
            })
            .ok()?;
        Some(())
    }

    pub fn resize_window(&mut self, id: WindowId, w: u32, h: u32) -> Option<()> {
        self.conn
            .configure_window(
                id,
                &ConfigureWindowAux::new().width(w as u32).height(h as u32),
            )
            .map_err(|e| {
                warn!("Failed to resize window {id}: {e}");
            })
            .ok()?;
        Some(())
    }

    pub fn grab_key(&mut self, mask: ModMask, key: u32) -> Option<()> {
        let masks = [
            mask,
            mask | ModMask::LOCK,
            mask | ModMask::M2,
            mask | ModMask::LOCK | ModMask::M2,
        ];

        for m in masks {
            self.conn
                .grab_key(
                    false,
                    self.screen.root,
                    m,
                    self.key_to_keycode(key) as u8,
                    GrabMode::ASYNC,
                    GrabMode::ASYNC,
                )
                .map_err(|e| {
                    warn!(
                        "Failed to grab key: {key:4X} with mask {m:2X}: {e}",
                        m = m.bits()
                    );
                })
                .ok()?;
        }
        Some(())
    }

    pub fn key_to_keycode(&self, c: u32) -> u32 {
        self.keymap.get(&(c)).copied().unwrap_or(0) as u32
    }
}
pub const XK_BACKSPACE: u32 = 0xff08;
pub const XK_RETURN: u32 = 0xff0d;
// pub const XK_RETURN:    u32 = 0x24;
pub const XK_ESCAPE: u32 = 0xff1b;
pub const XK_TAB: u32 = 0xff09;
pub const XK_SPACE: u32 = 0x0020;
