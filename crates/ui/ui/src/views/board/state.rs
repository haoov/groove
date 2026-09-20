//! What the board remembers between frames.

use groove_types::SessionId;

use crate::widget::Field;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct BoardUi {
    /// How far each column is scrolled, in pixels.
    pub live: f32,
    pub next: f32,
    pub review: f32,
    /// The Live items opened to show their worktrees.
    pub open: std::collections::BTreeSet<SessionId>,
    /// What the header's filter holds, and whether it takes what is typed.
    pub filter: Field,
    pub typing: bool,
    /// Which suggestion the keyboard stands on.
    pub offer: usize,
}

impl BoardUi {
    /// Shows a Live item's worktrees, or hides them again.
    pub fn fold(&mut self, id: &SessionId) {
        if !self.open.remove(id) {
            self.open.insert(id.clone());
        }
    }

    pub fn is_open(&self, id: &SessionId) -> bool {
        self.open.contains(id)
    }

    /// What the three columns are narrowed by.
    pub fn query(&self) -> super::filter::Query {
        super::filter::Query::of(self.filter.text())
    }

    /// The keyboard into the filter, at the end of what it holds.
    pub fn focus(&mut self) {
        self.typing = true;
        self.offer = 0;
        self.filter.end();
    }

    /// One offered row into the filter, in place of the token being typed.
    pub fn take(&mut self, pick: &str) {
        let text = super::complete::taken(self.filter.text(), pick);
        self.filter.set(&text);
        self.typing = true;
        self.offer = 0;
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
