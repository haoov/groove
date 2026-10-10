//! What an Application's resources view holds: the filter picked, and the groups folded.

use std::collections::BTreeSet;

use groove_types::{AppPart, AppResource};

/// Which of the Application's resources the view lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Filter {
    Attention,
    OutOfSync,
    Unhealthy,
    Syncing,
    All,
}

impl Filter {
    pub const OFFERED: [Filter; 5] = [
        Filter::Attention,
        Filter::OutOfSync,
        Filter::Unhealthy,
        Filter::Syncing,
        Filter::All,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Filter::Attention => "needs attention",
            Filter::OutOfSync => "out of sync",
            Filter::Unhealthy => "unhealthy",
            Filter::Syncing => "syncing",
            Filter::All => "all",
        }
    }

    /// Whether `one` stands in the list; the syncing view lists the sync's steps instead.
    pub fn holds(self, one: &AppResource) -> bool {
        match self {
            Filter::Attention => one.attention(),
            Filter::OutOfSync => one.out_of_sync(),
            Filter::Unhealthy => one.unhealthy(),
            Filter::Syncing => false,
            Filter::All => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ArgoUi {
    /// None is what needs attention, or all when nothing does.
    pub filter: Option<Filter>,
    /// The groups folded against how they start: under all, the healthy ones start folded.
    pub toggled: BTreeSet<String>,
    /// The column the rows of each group sort by, and whether descending.
    pub sort: Option<(String, bool)>,
}

impl ArgoUi {
    pub fn filter(&self, app: &AppPart) -> Filter {
        let running = app.operation.as_ref().is_some_and(|one| one.running());
        let attention = app.resources.iter().any(AppResource::attention);
        match self.filter {
            Some(filter) => filter,
            None if running => Filter::Syncing,
            None if attention => Filter::Attention,
            None => Filter::All,
        }
    }

    pub fn pick(&mut self, filter: Filter) {
        (self.filter, self.toggled) = (Some(filter), BTreeSet::new());
    }

    pub fn toggle(&mut self, group: &str) {
        if !self.toggled.remove(group) {
            self.toggled.insert(group.to_string());
        }
    }

    pub fn shut(&self, group: &str, folds: bool) -> bool {
        folds != self.toggled.contains(group)
    }
}
