use x11rb::{connection::Connection, protocol::xproto::{ChangeWindowAttributesAux, ConnectionExt, EventMask}, rust_connection::RustConnection};

pub struct X11RB {
    conn: RustConnection
}

impl X11RB {
    pub fn init(display_name: &str) -> Self {
        let (conn, screen_number) = x11rb::connect(Some(display_name)).unwrap();
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
            conn
        }
    }
}
