use x11rb::{connection::Connection, protocol::xproto::{ChangeWindowAttributesAux, ConnectionExt, EventMask, Screen}, rust_connection::RustConnection};

pub struct X11RB {
    conn: RustConnection,
    screen: Screen
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
}
