use x11rb::{connection::Connection, protocol::{Event, xproto::{ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt, EventMask, InputFocus, MappingNotifyEvent, Screen, StackMode, Time}}, rust_connection::RustConnection};

use log::warn;

pub type WindowId = u32;

pub struct X11RB {
    conn: RustConnection,
    screen: Screen,
    pointer_pos: (i16, i16)
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

        Self {
            screen: screen.clone(),
            conn,
            pointer_pos: (0, 0)
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
                self.conn.change_keyboard_mapping(e.count, e.first_keycode, 0, &[]);
            }
            ref e => {
                warn!("Ignoring x11 event: {e:?}");
            }
        };

        e
    }

    pub fn mouse_pos(&self) -> (i16, i16) {
        self.pointer_pos
    }

    pub fn update_mapping(&mut self, e: MappingNotifyEvent) {
        self.conn.change_keyboard_mapping(e.count, e.first_keycode, 0, &[]);
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
}
