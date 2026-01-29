use crate::WindowId;
use std::collections::HashMap;
#[derive(Debug, Copy, Clone, Default)]
pub struct Geometry {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
}

#[derive(Clone, Default)]
pub struct Workspace {
    windows: Vec<WindowId>,
    focused: Option<WindowId>,
    floating: HashMap<WindowId, Geometry>,
    full_screened: Option<WindowId>
}

impl Workspace {
    pub fn windows(&self) -> &[WindowId] {
        &self.windows
    }

    pub fn windows_mut(&mut self) -> &mut Vec<WindowId> {
        &mut self.windows
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    pub fn remove_window(&mut self, id: WindowId) {
        let was_focused = self.focused == Some(id);
        if let Some(p) = self.windows().iter().position(|i| id == *i) {
            self.windows.remove(p);
        } else {
            self.floating.remove(&id);
        }
        if was_focused {
            self.focused = self
                .windows
                .last()
                .copied()
                .or_else(|| self.floating.keys().next().copied());
        }
        if let Some(p) = self.windows.iter().position(|i| id == *i) {
            self.windows.remove(p);
            return;
        }
        self.floating.remove(&id);
    }

    pub fn floating_window_ids(&self) -> std::collections::hash_map::Keys<'_, u32, Geometry> {
        self.floating.keys()
    }

    pub fn set_focused_id(&mut self, id: WindowId) {
        self.focused = Some(id);
    }

    pub fn set_focused_to_newest_tiled_window(&mut self) {
        self.focused = self.windows.last().copied()
    }

    pub fn get_tiled_window_id(&self, index: usize) -> Option<&WindowId> {
        self.windows.get(index)
    }

    pub fn get_focused_id(&self) -> Option<WindowId> {
        self.focused
    }

    pub fn set_fullscreen_id(&mut self, id: Option<WindowId>) {
        self.full_screened = id;
    }

    pub fn floating_windows(&self) -> std::collections::hash_map::Iter<'_, u32, Geometry> {
        self.floating.iter()
    }

    pub fn get_fullscreen_id(&self) -> Option<WindowId> {
        self.full_screened
    }

    pub fn empty(&self) -> bool {
        self.windows.is_empty() && self.floating.is_empty()
    }

    pub fn push_window(&mut self, id: WindowId) {
        self.windows.push(id);
    }

    pub fn push_float_window(&mut self, id: WindowId, geometry: Geometry) {
        self.floating.insert(id, geometry);
    }

    pub fn get_geometry(&self, id: WindowId) -> Option<&Geometry> {
        self.floating.get(&id)
    }

    pub fn focus_tiled_left(&mut self) {
        if let Some(f) = self.focused
            && let Some(p) = self.windows().iter().position(|x| *x == f)
        {
            let new_pos = p.saturating_sub(1).clamp(0, self.window_count() - 1);
            self.focused = Some(self.windows[new_pos]);
        }
    }

    pub fn focus_tiled_right(&mut self) {
        if let Some(f) = self.focused
            && let Some(p) = self.windows().iter().position(|x| *x == f)
        {
            let new_pos = p.saturating_add(1).clamp(0, self.window_count() - 1);
            self.focused = Some(self.windows[new_pos]);
        }
    }

    pub fn tiled_swap_left(&mut self) {
        let f = match self.focused {
            Some(f) => f,
            None => return,
        };
        let pos = match self.windows.iter().position(|x| *x == f) {
            Some(p) => p,
            None => return,
        };
        if pos == 0 {
            return;
        }
        self.windows.swap(pos, pos - 1);
        self.focused = self.windows.get(pos - 1).copied();
    }

    pub fn tiled_swap_right(&mut self) {
        let f = match self.focused {
            Some(f) => f,
            None => return,
        };
        let pos = match self.windows.iter().position(|x| *x == f) {
            Some(p) => p,
            None => return,
        };
        if pos == self.window_count() - 1 {
            return;
        }
        self.windows.swap(pos, pos + 1);
        self.focused = self.windows.get(pos.saturating_add(1)).copied();
    }

    pub fn is_floating(&mut self, id: WindowId) -> bool {
        self.floating.contains_key(&id)
    }
}
