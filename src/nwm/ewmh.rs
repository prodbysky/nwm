
/// EWMH/ICCCM state manager
/// Ideally all the things to do with getting/setting atoms will be done here
use crate::WindowId;
use crate::better_x11rb;
use log::warn;
use x11rb::COPY_DEPTH_FROM_PARENT;
use x11rb::protocol::xproto::ClientMessageEvent;
use x11rb::protocol::xproto::ConnectionExt as OtherExt;
use x11rb::protocol::xproto::CreateWindowAux;
use x11rb::protocol::xproto::WindowClass;
use x11rb::{
    protocol::xproto::{Atom, AtomEnum, PropMode},
    wrapper::ConnectionExt, connection::Connection
};

pub struct Ewmh {
    window_type_atom: Option<Atom>,
    window_type_normal_atom: Option<Atom>,
    window_type_dock_atom: Option<Atom>,
    strut_partial_atom: Option<Atom>,
    active_desktop_atom: Option<Atom>,
    num_desktops_atom: Option<Atom>,
    active_window_atom: Option<Atom>,
    state_atom: Option<Atom>,
    fullscreen_state_atom: Option<Atom>,
}

impl Ewmh {
    /// Interns atoms that are needed for nwm functionality
    /// (fullscreen, docks, current active desktop)
    pub fn new(x11_ab: &mut better_x11rb::X11RB) -> Self {
        let mut supported_features = vec![];

        let supported_features_atom = x11_ab.intern_atom(b"_NET_SUPPORTED");
        if supported_features_atom.is_none() {
            warn!("Failed to intern _NET_SUPPORTED, windows ran within nwm won't know the supported features");
        }

        let window_type_atom = x11_ab.intern_atom(b"_NET_WM_WINDOW_TYPE");
        if window_type_atom.is_none() {
            warn!("Failed to intern _NET_WM_WINDOW_TYPE, emwh window type support is not present");
        } else {
            supported_features.push(window_type_atom.unwrap());
        }
        let window_type_dock_atom = x11_ab.intern_atom(b"_NET_WM_WINDOW_TYPE_DOCK");

        if window_type_dock_atom.is_none() {
            warn!(
                "Failed to intern _NET_WM_WINDOW_TYPE_DOCK, emwh window type support is not present"
            );
        } else {
            supported_features.push(window_type_dock_atom.unwrap());
        }

        let window_type_normal_atom = x11_ab.intern_atom(b"_NET_WM_WINDOW_TYPE_NORMAL");
        if window_type_normal_atom.is_none() {
            warn!(
                "Failed to intern _NET_WM_WINDOW_TYPE_NORMAL, emwh window type support is not present"
            );
        } else {
            supported_features.push(window_type_normal_atom.unwrap());
        }

        let strut_partial_atom = x11_ab.intern_atom(b"_NET_WM_STRUT_PARTIAL");
        if strut_partial_atom.is_none() {
            warn!(
                "Failed to intern _NET_WM_STRUT_PARTIAL, docks that depend on this won't resize other windows"
            );
        } else {
            supported_features.push(strut_partial_atom.unwrap());
        }

        let fullscreen_state_atom = x11_ab.intern_atom(b"_NET_WM_STATE_FULLSCREEN");
        if fullscreen_state_atom.is_none() {
            warn!(
                "Failed to intern _NET_WM_STATE_FULLSCREEN, fullscreen state will not be handled"
            );
        } else {
            supported_features.push(fullscreen_state_atom.unwrap());
        }

        let state_atom = x11_ab.intern_atom(b"_NET_WM_STATE");
        if state_atom.is_none() {
            warn!("Failed to intern _NET_WM_STATE, fullscreen state will not be handled");
        } else {
            supported_features.push(state_atom.unwrap());
        }

        let active_window_atom = x11_ab.intern_atom(b"_NET_ACTIVE_WINDOW");
        if state_atom.is_none() {
            warn!("Failed to intern _NET_ACTIVE_WINDOW");
        } else {
            supported_features.push(active_window_atom.unwrap());
            _ = x11_ab
                .conn
                .change_property32(
                    PropMode::REPLACE,
                    x11_ab.root_window(),
                    active_window_atom.unwrap(),
                    AtomEnum::CARDINAL,
                    &[x11rb::NONE],
                );
        }

        let num_desktops_atom = x11_ab.intern_atom(b"_NET_NUMBER_OF_DESKTOPS");
        if num_desktops_atom.is_none() {
            warn!("Failed to intern _NET_NUBER_OF_DESKTOPS");
        } else {
            supported_features.push(num_desktops_atom.unwrap());
            _ = x11_ab
                .conn
                .change_property32(
                    PropMode::REPLACE,
                    x11_ab.root_window(),
                    num_desktops_atom.unwrap(),
                    AtomEnum::CARDINAL,
                    &[10],
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
                    &[0],
                )
                .map_err(|e| {
                    warn!("Failed to set _NET_CURRENT_DESKTOP: {e}");
                });
            supported_features.push(at);
        }

        let support_check = x11_ab.intern_atom(b"_NET_SUPPORTING_WM_CHECK");
        if let Some(sc) = support_check {
            supported_features.push(sc);
            let win = x11_ab.conn.generate_id().unwrap();
            x11_ab.conn.create_window(
                COPY_DEPTH_FROM_PARENT,
                win,
                x11_ab.root_window(),
                0, 0, 1, 1,
                0,
                WindowClass::INPUT_OUTPUT,
                0,
                &CreateWindowAux::new(),
            ).unwrap();
            x11_ab.conn.change_property32(
                PropMode::REPLACE,
                x11_ab.root_window(),
                sc,
                AtomEnum::WINDOW,
                &[win],
            ).unwrap();
            x11_ab.conn.change_property32(
                PropMode::REPLACE,
                win,
                sc,
                AtomEnum::WINDOW,
                &[win],
            ).unwrap();
            let wm_name = x11_ab.intern_atom(b"_NET_WM_NAME").unwrap();
            let utf8 = x11_ab.intern_atom(b"UTF8_STRING").unwrap();
            x11_ab.conn.change_property8(
                PropMode::REPLACE,
                win,
                wm_name,
                utf8,
                b"nwm",
            ).unwrap();
        } else {
            warn!("Failed to intern _NET_SUPPORTING_WM_CHECK atom")
        }

        if let Some(sup) = supported_features_atom {
            _ = x11_ab.conn.change_property32(PropMode::REPLACE, x11_ab.root_window(), sup, AtomEnum::ATOM, &supported_features).map_err(|e| {
                warn!("Failed to change _NET_SUPPORTED property: {e}")
            });
        }



        Self {
            window_type_atom,
            window_type_dock_atom,
            window_type_normal_atom,
            state_atom,
            strut_partial_atom,
            fullscreen_state_atom,
            active_desktop_atom,
            num_desktops_atom,
            active_window_atom
        }
    }

    pub fn window_is_dock(&self, x11rb: &mut better_x11rb::X11RB, w: WindowId) -> bool {
        if self.window_type_dock_atom.is_none() {
            return false;
        }
        if let Some(types) = self.window_type(x11rb, w)
            && types.contains(&self.window_type_dock_atom.unwrap())
        {
            return true;
        }
        false
    }

    pub fn window_is_normal(&self, x11rb: &mut better_x11rb::X11RB, w: WindowId) -> bool {
        if self.window_type_normal_atom.is_none() {
            return false;
        }
        if let Some(types) = self.window_type(x11rb, w)
            && types.contains(&self.window_type_normal_atom.unwrap())
        {
            return true;
        }
        false
    }

    pub fn window_type(&self, x11rb: &mut better_x11rb::X11RB, w: WindowId) -> Option<Vec<u32>> {
        let rep = x11rb
            .conn
            .get_property(false, w, self.window_type_atom?, AtomEnum::ATOM, 0, 32).ok()?
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

    pub fn switch_active_desktop(&mut self, x11rb: &mut better_x11rb::X11RB, num: usize) {
        if let Some(active) = self.active_desktop_atom {
            _ = x11rb
                .conn
                .change_property32(
                    PropMode::REPLACE,
                    x11rb.root_window(),
                    active,
                    AtomEnum::CARDINAL,
                    &[num as u32],
                )
                .map_err(|e| {
                    warn!("Failed to set _NET_CURRENT_DESKTOP: {e}");
                });
        }
    }

    pub fn set_focused(&mut self, id: WindowId, x11_ab: &mut better_x11rb::X11RB) {
        if let Some(focused) = self.active_window_atom {
            _ = x11_ab
                .conn
                .change_property32(
                    PropMode::REPLACE,
                    x11_ab.root_window(),
                    focused,
                    AtomEnum::CARDINAL,
                    &[id as u32],
                )
                .map_err(|e| {
                    warn!("Failed to set _NET_CURRENT_DESKTOP: {e}");
                });
        }
    }

    pub fn get_strut(&mut self, x11rb: &mut better_x11rb::X11RB, w: WindowId) -> Option<Strut> {
        let rep = x11rb
            .conn
            .get_property(
                false,
                w,
                self.strut_partial_atom?,
                AtomEnum::CARDINAL,
                0,
                12,
            )
            .map_err(|e| {
                warn!("Failed to get _NET_WM_STRUT_PARTIAL property: {e}");
            })
            .ok()?
            .reply()
            .map_err(|e| {
                warn!(
                    "Failed to get _NET_WM_STRUT_PARTIAL property reply from the x11 server: {e}"
                );
            })
            .ok()?;

        let values = rep.value32()?.collect::<Vec<_>>();

        if values.len() < 12 {
            return None;
        }

        let mut arr = [0u32; 12];

        arr.copy_from_slice(&values[..12]);
        Some(Strut::from(arr))
    }

    /// Called when a x11rb::Event::ClientMessageEvent is sent to check for fullscreen state.
    /// That being said it can be more than this but I'm willing to ignore that for some time
    pub fn get_fullscreen_msg(&mut self, e: ClientMessageEvent) -> Option<FullscreenMessage> {
        if let Some(sa) = self.state_atom
            && let Some(fsa) = self.fullscreen_state_atom
        {
            if e.type_ == sa {
                let (action, first, second) = (
                    e.data.as_data32()[0],
                    e.data.as_data32()[1],
                    e.data.as_data32()[2],
                );
                if first == fsa || second == fsa {
                    match action {
                        0 => {
                            return Some(FullscreenMessage::DisableFullscreen);
                        }
                        1 => {
                            return Some(FullscreenMessage::EnableFullscreen);
                        }
                        2 => {
                            return Some(FullscreenMessage::ToggleFullscreen);
                        }
                        _ => {
                            return None;
                        }
                    }
                }
            }
        }
        None
    }
}

pub enum FullscreenMessage {
    DisableFullscreen,
    EnableFullscreen,
    ToggleFullscreen,
}

#[allow(dead_code)]
pub struct Strut {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
    pub left_start_y: u32,
    pub left_end_y: u32,
    pub right_start_y: u32,
    pub right_end_y: u32,
    pub top_start_x: u32,
    pub top_end_x: u32,
    pub bottom_start_x: u32,
    pub bottom_end_x: u32,
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
