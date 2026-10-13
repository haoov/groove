//! What an Application's resources view holds: the filter picked, and the groups folded.

use groove_types::{AppPart, AppResource};

/// Which of the Application's resources the view lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Filter {
    Attention,
    OutOfSync,
    Syncing,
    All,
}

impl Filter {
    pub const OFFERED: [Filter; 4] = [
        Filter::Attention,
        Filter::OutOfSync,
        Filter::Syncing,
        Filter::All,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Filter::Attention => "needs attention",
            Filter::OutOfSync => "out of sync",
            Filter::Syncing => "syncing",
            Filter::All => "all",
        }
    }

    /// Whether `one` stands in the list; the syncing view lists the sync's steps instead.
    pub fn holds(self, part: &AppPart, one: &AppResource) -> bool {
        match self {
            Filter::Attention => part.attention(one),
            Filter::OutOfSync => one.out_of_sync(),
            Filter::Syncing => false,
            Filter::All => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ArgoUi {
    /// None is what needs attention, or all when nothing does.
    pub filter: Option<Filter>,
}

impl ArgoUi {
    pub fn filter(&self, app: &AppPart) -> Filter {
        let running = app.operation.as_ref().is_some_and(|one| one.running());
        let attention = app.resources.iter().any(|one| app.attention(one));
        match self.filter {
            Some(filter) => filter,
            None if running => Filter::Syncing,
            None if attention => Filter::Attention,
            None => Filter::All,
        }
    }
}
