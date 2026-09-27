/// A rectangle in pixels, top-left origin.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn right(self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(self) -> f32 {
        self.y + self.h
    }

    pub fn is_empty(self) -> bool {
        self.w <= 0.0 || self.h <= 0.0
    }

    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.right() && y < self.bottom()
    }

    pub fn intersect(self, other: Rect) -> Rect {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        Rect::new(x, y, (right - x).max(0.0), (bottom - y).max(0.0))
    }

    pub fn inset(self, d: f32) -> Rect {
        Rect::new(self.x + d, self.y + d, self.w - 2.0 * d, self.h - 2.0 * d)
    }

    pub fn pad(self, edges: Edges) -> Rect {
        let w = (self.w - edges.left - edges.right).max(0.0);
        let h = (self.h - edges.top - edges.bottom).max(0.0);
        Rect::new(self.x + edges.left, self.y + edges.top, w, h)
    }

    /// What lies left of `x`.
    pub fn until(self, x: f32) -> Rect {
        Rect::new(self.x, self.y, (x - self.x).clamp(0.0, self.w), self.h)
    }

    /// The first `w` of the width; `self` keeps the rest.
    pub fn take_left(&mut self, w: f32) -> Rect {
        let w = w.clamp(0.0, self.w);
        let taken = Rect::new(self.x, self.y, w, self.h);
        self.x += w;
        self.w -= w;
        taken
    }

    /// The last `w` of the width; `self` keeps the rest.
    pub fn take_right(&mut self, w: f32) -> Rect {
        let w = w.clamp(0.0, self.w);
        self.w -= w;
        Rect::new(self.right(), self.y, w, self.h)
    }

    /// The first `h` of the height; `self` keeps the rest.
    pub fn take_top(&mut self, h: f32) -> Rect {
        let h = h.clamp(0.0, self.h);
        let taken = Rect::new(self.x, self.y, self.w, h);
        self.y += h;
        self.h -= h;
        taken
    }

    /// The last `h` of the height; `self` keeps the rest.
    pub fn take_bottom(&mut self, h: f32) -> Rect {
        let h = h.clamp(0.0, self.h);
        self.h -= h;
        Rect::new(self.x, self.bottom(), self.w, h)
    }

    pub fn align(self, (w, h): (f32, f32), across: Align, down: Align) -> Rect {
        Rect::new(
            across.at(self.x, self.w, w),
            down.at(self.y, self.h, h),
            w,
            h,
        )
    }

    /// The whole-pixel scissor box this covers inside `size`.
    pub(crate) fn scissor(self, size: Size) -> Option<(u32, u32, u32, u32)> {
        let r = self.intersect(size.rect());
        if r.is_empty() {
            return None;
        }
        let x0 = r.x.floor() as u32;
        let y0 = r.y.floor() as u32;
        let x1 = (r.right().ceil() as u32).min(size.width);
        let y1 = (r.bottom().ceil() as u32).min(size.height);
        (x1 > x0 && y1 > y0).then_some((x0, y0, x1 - x0, y1 - y0))
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Edges {
    pub const fn all(d: f32) -> Self {
        Self::xy(d, d)
    }

    pub const fn xy(x: f32, y: f32) -> Self {
        Self {
            top: y,
            right: x,
            bottom: y,
            left: x,
        }
    }

    pub const fn across(left: f32, right: f32) -> Self {
        Self {
            top: 0.0,
            right,
            bottom: 0.0,
            left,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
}

impl Align {
    fn at(self, from: f32, room: f32, size: f32) -> f32 {
        match self {
            Align::Start => from,
            Align::Center => from + (room - size) / 2.0,
            Align::End => from + room - size,
        }
    }
}

/// A size in whole pixels.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn rect(self) -> Rect {
        Rect::new(0.0, 0.0, self.width as f32, self.height as f32)
    }

    pub(crate) fn clamped(self) -> Self {
        Self::new(self.width.max(1), self.height.max(1))
    }
}
