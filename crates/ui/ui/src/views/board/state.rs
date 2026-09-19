//! What the board remembers between frames.

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct BoardUi {
    /// How far each column is scrolled, in pixels.
    pub live: f32,
    pub next: f32,
    pub review: f32,
}

impl BoardUi {
    pub fn scroll(&self, list: super::List) -> f32 {
        match list {
            super::List::Live => self.live,
            super::List::Next => self.next,
            super::List::Review => self.review,
        }
    }

    pub fn scrolled(&mut self, list: super::List, to: f32) {
        match list {
            super::List::Live => self.live = to,
            super::List::Next => self.next = to,
            super::List::Review => self.review = to,
        }
    }
}
