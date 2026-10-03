//! Which column orders the review table, and which way.

use std::cmp::Ordering;

use groove_types::ReviewMr;
use groove_ui_kit::widgets::Sorted;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum By {
    Title,
    Updated,
}

impl By {
    pub const ALL: [By; 2] = [By::Title, By::Updated];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Order {
    pub by: By,
    pub descending: bool,
}

/// The last updated first.
impl Default for Order {
    fn default() -> Self {
        Self {
            by: By::Updated,
            descending: true,
        }
    }
}

impl Order {
    /// The same column turns the order round; another starts on its own default.
    pub fn clicked(self, by: By) -> Self {
        match by == self.by {
            true => Self {
                descending: !self.descending,
                ..self
            },
            false => Self {
                by,
                descending: by == By::Updated,
            },
        }
    }

    pub fn sorted(self) -> Sorted {
        let column = By::ALL.iter().position(|by| *by == self.by).unwrap_or(0);
        Sorted {
            column,
            descending: self.descending,
        }
    }

    pub fn sort(self, mrs: &mut [&ReviewMr]) {
        mrs.sort_by(|one, two| {
            let order = compared(self.by, one, two);
            match self.descending {
                true => order.reverse(),
                false => order,
            }
        });
    }
}

fn compared(by: By, one: &ReviewMr, two: &ReviewMr) -> Ordering {
    match by {
        By::Title => one.title.to_lowercase().cmp(&two.title.to_lowercase()),
        By::Updated => one.updated_at.cmp(&two.updated_at),
    }
}
