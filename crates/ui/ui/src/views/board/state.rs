//! What the board remembers between frames.

#[derive(Debug, Default, Clone, PartialEq)]
pub struct BoardUi {
    /// How far each column is scrolled, in pixels.
    pub live: f32,
    pub next: f32,
    pub review: f32,
    /// The Live items opened to show their worktrees.
    pub open: std::collections::BTreeSet<groove_types::SessionId>,
}

impl BoardUi {
    /// Shows a Live item's worktrees, or hides them again.
    pub fn fold(&mut self, id: &groove_types::SessionId) {
        if !self.open.remove(id) {
            self.open.insert(id.clone());
        }
    }

    pub fn is_open(&self, id: &groove_types::SessionId) -> bool {
        self.open.contains(id)
    }
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
