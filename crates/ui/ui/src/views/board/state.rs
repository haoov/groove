//! What the board remembers between frames.

use groove_ui_kit::widgets::Field;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct BoardUi {
    /// How far each column is scrolled, in pixels.
    pub live: f32,
    pub next: f32,
    pub review: f32,
    /// What the header's filter holds, and whether it takes what is typed.
    pub filter: Field,
    pub typing: bool,
    /// Which suggestion the keyboard stands on.
    pub offer: usize,
    /// The line a carried task would land on.
    pub drop: Option<usize>,
    pub reviews: super::review::Order,
    /// The review the keyboard stands on, by its project and number.
    pub chosen: Option<(String, u64)>,
}

impl BoardUi {
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
