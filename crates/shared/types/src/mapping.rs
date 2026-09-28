//! What a source holds, and how Groove's names and values are mapped onto it.

use crate::{
    GithubConfig, NotionConfig, Priority, PriorityMap, PropertyNames, StatusIntent, StatusMap,
};

/// A property's type, as far as the mapping tells them apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Status,
    Select,
    Date,
    Number,
    People,
    Relation,
    Other,
}

/// One property of a source, and the values it offers when it is a choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    pub name: String,
    pub kind: Kind,
    pub options: Vec<String>,
}

/// A name Groove reads from a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mapped {
    Status,
    Priority,
    Start,
    Due,
    Estimate,
    Logged,
    Assignee,
    Sprint,
}

impl Mapped {
    pub const NOTION: [Mapped; 8] = [
        Mapped::Assignee,
        Mapped::Sprint,
        Mapped::Status,
        Mapped::Priority,
        Mapped::Start,
        Mapped::Due,
        Mapped::Estimate,
        Mapped::Logged,
    ];
    pub const GITHUB: [Mapped; 6] = [
        Mapped::Status,
        Mapped::Priority,
        Mapped::Start,
        Mapped::Due,
        Mapped::Estimate,
        Mapped::Logged,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Mapped::Status => "status",
            Mapped::Priority => "priority",
            Mapped::Start => "start",
            Mapped::Due => "due",
            Mapped::Estimate => "estimate",
            Mapped::Logged => "logged",
            Mapped::Assignee => "assignee",
            Mapped::Sprint => "sprint",
        }
    }

    /// What its gap costs.
    pub fn cost(self) -> &'static str {
        match self {
            Mapped::Status => "a task cannot move",
            Mapped::Priority => "no priority on the board",
            Mapped::Start => "no start date on the plan",
            Mapped::Due => "no due date, no due-soon warning",
            Mapped::Estimate => "no estimate beside the time",
            Mapped::Logged => "the hours measured are not logged",
            Mapped::Assignee | Mapped::Sprint => "no task is listed",
        }
    }

    /// The types a property must have to be mapped to it.
    pub fn fits(self, kind: Kind) -> bool {
        let kinds: &[Kind] = match self {
            Mapped::Status | Mapped::Priority => &[Kind::Status, Kind::Select],
            Mapped::Start | Mapped::Due => &[Kind::Date],
            Mapped::Estimate | Mapped::Logged => &[Kind::Number],
            Mapped::Assignee => &[Kind::People],
            Mapped::Sprint => &[Kind::Relation],
        };
        kinds.contains(&kind)
    }
}

/// One change to a source's mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mapping {
    /// The property a name reads; its values start unmapped again.
    Name(Mapped, String),
    /// The source's value for one of Groove's statuses, or priorities.
    Status(StatusIntent, String),
    Priority(Priority, String),
}

impl PriorityMap {
    /// The source's value for a level.
    pub fn value(&self, level: Priority) -> Option<&str> {
        let values = match level {
            Priority::High => &self.high,
            Priority::Medium => &self.medium,
            Priority::Low => &self.low,
        };
        values.first().map(String::as_str)
    }
}

impl NotionConfig {
    /// The property a name reads, or `None` while it is a gap.
    pub fn name(&self, which: Mapped) -> Option<&str> {
        match which {
            Mapped::Assignee => self.assignee(),
            Mapped::Sprint => self.sprint(),
            other => named(&self.properties, other),
        }
    }

    pub fn map(&mut self, change: &Mapping) {
        match change {
            Mapping::Name(Mapped::Assignee, name) => self.assignee = Some(name.clone()),
            Mapping::Name(Mapped::Sprint, name) => self.sprint = Some(name.clone()),
            Mapping::Name(Mapped::Status, _) => self.filters.exclude_statuses.clear(),
            _ => {}
        }
        mapped(
            &mut self.properties,
            &mut self.status_map,
            &mut self.priority_map,
            change,
        );
    }
}

impl GithubConfig {
    pub fn name(&self, which: Mapped) -> Option<&str> {
        named(&self.properties, which)
    }

    pub fn map(&mut self, change: &Mapping) {
        mapped(
            &mut self.properties,
            &mut self.status_map,
            &mut self.priority_map,
            change,
        );
    }
}

/// The names and the two value maps every source shares.
fn mapped(
    names: &mut PropertyNames,
    status: &mut StatusMap,
    priority: &mut PriorityMap,
    change: &Mapping,
) {
    match change {
        Mapping::Name(which, name) => renamed(names, (status, priority), *which, name),
        Mapping::Status(intent, value) => {
            let list = match intent {
                StatusIntent::Ready => &mut status.ready,
                StatusIntent::InProgress => &mut status.in_progress,
                StatusIntent::Done => &mut status.done,
            };
            *list = vec![value.clone()];
        }
        Mapping::Priority(level, value) => {
            let list = match level {
                Priority::High => &mut priority.high,
                Priority::Medium => &mut priority.medium,
                Priority::Low => &mut priority.low,
            };
            *list = vec![value.clone()];
        }
    }
}

/// A name pointed at another property; a status or priority forgets its values.
fn renamed(
    names: &mut PropertyNames,
    (status, priority): (&mut StatusMap, &mut PriorityMap),
    which: Mapped,
    name: &str,
) {
    let named = Some(name.to_string());
    match which {
        Mapped::Status => {
            names.status = name.to_string();
            *status = StatusMap::default();
        }
        Mapped::Priority => {
            names.priority = named;
            *priority = PriorityMap::default();
        }
        Mapped::Start => names.start = named,
        Mapped::Due => names.due = named,
        Mapped::Estimate => names.estimate = named,
        Mapped::Logged => names.logged = named,
        Mapped::Assignee | Mapped::Sprint => {}
    }
}

fn named(names: &PropertyNames, which: Mapped) -> Option<&str> {
    let name = match which {
        Mapped::Status => Some(names.status.as_str()),
        Mapped::Priority => names.priority.as_deref(),
        Mapped::Start => names.start.as_deref(),
        Mapped::Due => names.due.as_deref(),
        Mapped::Estimate => names.estimate.as_deref(),
        Mapped::Logged => names.logged.as_deref(),
        Mapped::Assignee | Mapped::Sprint => None,
    };
    name.filter(|one| !one.trim().is_empty())
}
