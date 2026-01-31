use crate::{WindowId, Rect, Reserve};

pub trait Layout {
    /// Move and rearrange windows
    fn arrange(&self, ctx: &LayoutContext) -> Vec<(WindowId, Rect)>;
    /// Get the window to focus when moving in a direction
    fn focus_next(&self, ctx: &LayoutContext, current: WindowId, dir: Direction) -> Option<WindowId>;
    
    /// Get the new window order after swapping in a direction
    fn swap(&self, ctx: &LayoutContext, current: WindowId, dir: Direction) -> Option<Vec<WindowId>>;

}

pub enum Direction {
    Left,
    Right,
    Up,
    Down
}

pub struct LayoutContext<'a> {
    pub windows: &'a [WindowId],
    pub screen_width: u16,
    pub screen_height: u16,
    pub gap: u8,
    pub reserved: Reserve,
}

pub struct HorizontalTiling;
pub struct VerticalTiling;

impl Layout for HorizontalTiling {
    fn arrange(&self, ctx: &LayoutContext) -> Vec<(WindowId, Rect)> {
        let n = ctx.windows.len() as i16;
        if n == 0 {
            return vec![];
        }

        let mut rects = Vec::new();
        let gap = ctx.gap as i16;
        let half_gap = gap / 2;

        let offset_x = ctx.reserved.x0 as i16;
        let offset_y = ctx.reserved.y0 as i16;
        let width = ctx.screen_width as i16 - (ctx.reserved.x0 + ctx.reserved.x1) as i16;
        let height = ctx.screen_height as i16 - (ctx.reserved.y0 + ctx.reserved.y1) as i16;

        let usable_w = width - gap * 2;
        let slot_w = usable_w / n;

        for (i, &window) in ctx.windows.iter().enumerate() {
            let x = gap + (i as i16) * slot_w + half_gap + offset_x;
            let y = gap + offset_y;
            let w = slot_w - gap;
            let h = height - gap * 2;

            if w > 0 && h > 0 {
                rects.push((window, Rect { x, y, w, h }));
            }
        }

        rects
    }

    fn focus_next(&self, ctx: &LayoutContext, current: WindowId, dir: Direction) -> Option<WindowId> {
        let pos = ctx.windows.iter().position(|x| *x == current)?;
        match dir {
            Direction::Up | Direction::Down => None,
            Direction::Left => {
                if pos > 0 {
                    ctx.windows.get(pos - 1).copied()
                } else {
                    None
                }
            }
            Direction::Right => {
                if pos < ctx.windows.len() - 1 {
                    ctx.windows.get(pos + 1).copied()
                } else {
                    None
                }
            }
        }
    }

    fn swap(&self, ctx: &LayoutContext, current: WindowId, dir: Direction) -> Option<Vec<WindowId>> {
        let pos = ctx.windows.iter().position(|&w| w == current)?;
        let mut new_order = ctx.windows.to_vec();
        
        match dir {
            Direction::Left => {
                if pos > 0 {
                    new_order.swap(pos, pos - 1);
                    Some(new_order)
                } else {
                    None
                }
            }
            Direction::Right => {
                if pos < new_order.len() - 1 {
                    new_order.swap(pos, pos + 1);
                    Some(new_order)
                } else {
                    None
                }
            }
            Direction::Up | Direction::Down => None,
        }
    }
}

impl Layout for VerticalTiling {
    fn arrange(&self, ctx: &LayoutContext) -> Vec<(WindowId, Rect)> {
        let mut rects = vec![];
        let n = ctx.windows.len() as i16;
        if n == 0 { return rects; }
        
        let sw = ctx.screen_width as i16 - (ctx.reserved.x0 + ctx.reserved.x1) as i16;
        let sh = ctx.screen_height as i16 - (ctx.reserved.y0 + ctx.reserved.y1) as i16;
        let gap = ctx.gap as i16;
        let offset_x = ctx.reserved.x0 as i16;
        let offset_y = ctx.reserved.y0 as i16;
        
        let usable_h = sh - gap * 2;
        let slot_h = usable_h / n;
        
        for (i, &win) in ctx.windows.iter().enumerate() {
            let x = gap + offset_x;
            let y = gap + (i as i16) * slot_h + gap / 2 + offset_y;
            let w = sw - gap * 2;
            let h = slot_h - gap;
            
            if w > 0 && h > 0 {
                rects.push((win, Rect { x, y, w, h }));
            }
        }
        
        rects
    }

    fn focus_next(&self, ctx: &LayoutContext, current: WindowId, dir: Direction) -> Option<WindowId> {
        let pos = ctx.windows.iter().position(|x| *x == current)?;
        match dir {
            Direction::Left | Direction::Right => None,
            Direction::Up => {
                if pos > 0 {
                    ctx.windows.get(pos - 1).copied()
                } else {
                    None
                }
            }
            Direction::Down => {
                if pos < ctx.windows.len() - 1 {
                    ctx.windows.get(pos + 1).copied()
                } else {
                    None
                }
            }
        }
    }

    fn swap(&self, ctx: &LayoutContext, current: WindowId, dir: Direction) -> Option<Vec<WindowId>> {
        let pos = ctx.windows.iter().position(|&w| w == current)?;
        let mut new_order = ctx.windows.to_vec();
        
        match dir {
            Direction::Up => {
                if pos > 0 {
                    new_order.swap(pos, pos - 1);
                    Some(new_order)
                } else {
                    None
                }
            }
            Direction::Down => {
                if pos < new_order.len() - 1 {
                    new_order.swap(pos, pos + 1);
                    Some(new_order)
                } else {
                    None
                }
            }
            Direction::Left | Direction::Right => None,
        }
    }
}
