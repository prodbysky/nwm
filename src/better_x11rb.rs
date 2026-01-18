use std::collections::HashMap;

use x11rb::{connection::Connection, protocol::{Event, xproto::{ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt, EventMask, GrabMode, InputFocus, Keycode, MappingNotifyEvent, ModMask, Screen, StackMode, Time}}, rust_connection::RustConnection};

use log::warn;

pub type WindowId = u32;

pub struct X11RB {
    conn: RustConnection,
    screen: Screen,
    pointer_pos: (i16, i16),
    keymap: HashMap<u32, Keycode>,
}

impl X11RB {
    pub fn init() -> Self {
        let (conn, screen_number) = x11rb::connect(None).unwrap();
        let screen = &conn.setup().roots[screen_number];
        let root = screen.root;

        let event_mask = 
            EventMask::SUBSTRUCTURE_REDIRECT |
            EventMask::SUBSTRUCTURE_NOTIFY |
            EventMask::KEY_PRESS |
            EventMask::POINTER_MOTION;

        conn.change_window_attributes(
            root, 
            &ChangeWindowAttributesAux::new().event_mask(event_mask),
        ).unwrap();

        let mut wm = 
        Self {
            screen: screen.clone(),
            conn,
            pointer_pos: (0, 0),
            keymap: HashMap::new()
        };
        wm.rebuild_keymap();

        wm
    }

    fn rebuild_keymap(&mut self) {
        self.keymap.clear();

        let setup = self.conn.setup();
        let min = setup.min_keycode;
        let max = setup.max_keycode;

        let reply = self.conn
            .get_keyboard_mapping(min, max - min + 1)
            .unwrap()
            .reply()
            .unwrap();

        let per = reply.keysyms_per_keycode as usize;

        for (i, syms) in reply.keysyms.chunks(per).enumerate() {
            let keycode = min + i as u8;
            for &keysym in syms {
                // if (0x20..=0x7e).contains(&keysym) {
                    self.keymap.insert(keysym, keycode);
                // }
            }
        }
    }

    pub fn grab_pointer(&mut self) {
        self.conn.grab_pointer(
            true, 
            self.screen.root,
            x11rb::protocol::xproto::EventMask::POINTER_MOTION |
            x11rb::protocol::xproto::EventMask::ENTER_WINDOW,
            x11rb::protocol::xproto::GrabMode::ASYNC, 
            x11rb::protocol::xproto::GrabMode::ASYNC, 
            0_u32, 
            0_u32,
            x11rb::protocol::xproto::Time::CURRENT_TIME
        );
        self.conn.flush();
    }

    pub fn next_event(&mut self) -> Event {
        let e = self.conn.wait_for_event().unwrap();

        match e {
            Event::MotionNotify(e) => {
                self.pointer_pos.0 = e.root_x;
                self.pointer_pos.1 = e.root_y;
            }
            Event::MappingNotify(e) => {
                // NOTE: I can't actually understand this if stuff breaks look at THIS
                self.update_mapping(e); 
            }
            _ => {

            }
        };

        e
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

    pub fn raise_window(&mut self, id: WindowId) {
        self.conn.map_window(id);
        self.conn.configure_window(id, &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE));
        self.conn.flush();
    }

    pub fn map_window(&mut self, id: WindowId) {
        self.conn.map_window(id);
        self.conn.flush();
    }

    pub fn unmap_window(&mut self, id: WindowId) {
        self.conn.unmap_window(id);
        self.conn.flush();

    }

    pub fn focus_window(&mut self, id: WindowId) {
        self.conn.set_input_focus(InputFocus::POINTER_ROOT, id, Time::CURRENT_TIME);
        self.conn.flush();
    }

    pub fn close_window(&mut self, id: WindowId) {
        self.conn.destroy_window(id);
        self.conn.flush();
    }

    pub fn move_window(&mut self, id: WindowId, x: i16, y: i16) {
        self.conn.configure_window(id, &ConfigureWindowAux::new().x(x as i32).y(y as i32));
        self.conn.flush();
    }

    pub fn resize_window(&mut self, id: WindowId, w: u32, h: u32) {
        self.conn.configure_window(id, &ConfigureWindowAux::new().width(w as u32).height(h as u32));
        self.conn.flush();
    }

    pub fn grab_key(&mut self, mask: ModMask, key: u32) {
        let masks = [
            mask,
            mask | ModMask::LOCK,
            mask | ModMask::M2,
            mask | ModMask::LOCK | ModMask::M2,
        ];

        for m in masks {
            self.conn.grab_key(false, self.screen.root, m, self.key_to_keycode(key) as u8, GrabMode::ASYNC, GrabMode::ASYNC);
        }
        self.conn.flush();
    }

    pub fn key_to_keycode(&self, c: u32) -> u32 {
        self.keymap.get(&(c)).copied().unwrap_or(0) as u32
    }
}
pub const XK_BACKSPACE: u32 = 0xff08;
pub const XK_RETURN:    u32 = 0xff0d;
// pub const XK_RETURN:    u32 = 0x24;
pub const XK_ESCAPE:    u32 = 0xff1b;
pub const XK_TAB:       u32 = 0xff09;
pub const XK_SPACE:     u32 = 0x0020;

