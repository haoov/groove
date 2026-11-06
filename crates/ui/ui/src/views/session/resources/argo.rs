//! What an Application's resources view holds: the filter picked, and the groups folded.

use groove_types::{AppPart, AppResource, ResourceHealth, health_of};

/// Which of the Application's resources the view lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Filter {
    Degraded,
    OutOfSync,
    Syncing,
    All,
}

impl Filter {
    pub const OFFERED: [Filter; 4] = [
        Filter::Degraded,
        Filter::OutOfSync,
        Filter::Syncing,
        Filter::All,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Filter::Degraded => "degraded",
            Filter::OutOfSync => "out of sync",
            Filter::Syncing => "syncing",
            Filter::All => "all",
        }
    }

    /// Whether `one` stands in the list; the syncing view lists the sync's steps instead.
    pub fn holds(self, health: &[ResourceHealth], one: &AppResource) -> bool {
        match self {
            Filter::Degraded => health_of(health, one) == "Degraded",
            Filter::OutOfSync => one.out_of_sync(),
            Filter::Syncing => false,
            Filter::All => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ArgoUi {
    /// None opens on the sync running, else the first of degraded and out of sync to hold any.
    pub filter: Option<Filter>,
}

impl ArgoUi {
    pub fn filter(&self, app: &AppPart, health: &[ResourceHealth]) -> Filter {
        let running = app.operation.as_ref().is_some_and(|one| one.running());
        let any = |filter: Filter| app.resources.iter().any(|one| filter.holds(health, one));
        match self.filter {
            Some(filter) => filter,
            None if running => Filter::Syncing,
            None if any(Filter::Degraded) => Filter::Degraded,
            None if any(Filter::OutOfSync) => Filter::OutOfSync,
            None => Filter::All,
        }
    }
}
