//! A source's mapping: each name Groove reads, then the source's value for each of Groove's
//! statuses and priorities, every one picked from what the source holds.

use groove_controllers::AppState;
use groove_types::{
    EstimateUnit, GithubConfig, Mapped, Mapping, NotionConfig, Priority, Property, ProviderId,
    StatusIntent,
};
use groove_ui_kit::base::style::Role;

use super::{Row, Section, Value};
use crate::hit::Target;

/// One thing Groove maps onto a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Name(Mapped),
    Status(StatusIntent),
    Priority(Priority),
    /// What the estimate counts in.
    Unit,
}

const STATUSES: [(StatusIntent, &str); 3] = [
    (StatusIntent::Ready, "ready"),
    (StatusIntent::InProgress, "in progress"),
    (StatusIntent::Done, "done"),
];

/// The source whose mapping the rows show.
pub(super) enum Held<'a> {
    Notion(&'a NotionConfig),
    Github(&'a GithubConfig),
}

impl Held<'_> {
    fn id(&self) -> ProviderId {
        match self {
            Held::Notion(_) => ProviderId::Notion,
            Held::Github(_) => ProviderId::Github,
        }
    }

    fn names(&self) -> &'static [Mapped] {
        match self {
            Held::Notion(_) => &Mapped::NOTION,
            Held::Github(_) => &Mapped::GITHUB,
        }
    }

    /// What a slot holds now, or `None` while it is a gap.
    fn held(&self, slot: Slot) -> Option<&str> {
        let (status, priority, names) = match self {
            Held::Notion(one) => (&one.status_map, &one.priority_map, &one.properties),
            Held::Github(one) => (&one.status_map, &one.priority_map, &one.properties),
        };
        match (self, slot) {
            (Held::Notion(one), Slot::Name(which)) => one.name(which),
            (Held::Github(one), Slot::Name(which)) => one.name(which),
            (_, Slot::Status(intent)) => status.label(intent),
            (_, Slot::Priority(level)) => priority.value(level),
            (_, Slot::Unit) => Some(names.estimate_unit.label()),
        }
    }
}

/// What a slot's menu offers: the source's own names, what is held now, and what it says empty.
#[derive(Debug, Clone, PartialEq)]
pub struct Choices {
    pub source: ProviderId,
    pub slot: Slot,
    pub options: Vec<String>,
    pub empty: &'static str,
}

impl Choices {
    /// The change picking one row makes.
    pub fn change(&self, at: usize) -> Option<Mapping> {
        let name = self.options.get(at)?.clone();
        Some(match self.slot {
            Slot::Name(which) => Mapping::Name(which, name),
            Slot::Status(intent) => Mapping::Status(intent, name),
            Slot::Priority(level) => Mapping::Priority(level, name),
            Slot::Unit => Mapping::Unit(EstimateUnit::parse(&name)?),
        })
    }
}

/// The menu of one slot of one source, as the source was last read.
pub fn choices(app: &AppState, source: ProviderId, slot: Slot) -> Choices {
    let config = app.config.config.as_ref();
    let held = match source {
        ProviderId::Notion => config.and_then(|c| c.notion.as_ref()).map(Held::Notion),
        ProviderId::Github => config.and_then(|c| c.github.as_ref()).map(Held::Github),
    };
    let schema = app.config.schema(source);
    let options = match (&held, schema) {
        (Some(held), Some(schema)) => offered(held, slot, schema),
        (Some(held), None) if slot == Slot::Unit => offered(held, slot, &[]),
        _ => Vec::new(),
    };
    let empty = match (schema, app.config.reading.contains(&source)) {
        (None, true) => "reading the source…",
        (None, false) => "the source is not read yet",
        (Some(_), _) => "nothing of that type",
    };
    Choices {
        source,
        slot,
        options,
        empty,
    }
}

/// The mapping in three blocks: the names Groove reads, then the status and the priority values.
pub(super) struct Blocks {
    pub fields: Vec<Row>,
    pub statuses: Vec<Row>,
    pub priorities: Vec<Row>,
}

pub(super) fn mapping(held: &Held) -> Blocks {
    let row = |(slot, label)| slotted(held, slot, label);
    let mut fields = Vec::new();
    for which in held.names() {
        fields.push(row((Slot::Name(*which), which.label())));
        if *which == Mapped::Estimate && held.held(Slot::Name(*which)).is_some() {
            fields.push(row((Slot::Unit, "estimate unit")));
        }
    }
    let mapped =
        |which: Mapped| held.names().contains(&which) && held.held(Slot::Name(which)).is_some();
    let statuses = match mapped(Mapped::Status) {
        true => STATUSES
            .iter()
            .map(|(one, label)| row((Slot::Status(*one), label)))
            .collect(),
        false => Vec::new(),
    };
    let priorities = match mapped(Mapped::Priority) {
        true => Priority::ALL
            .iter()
            .map(|one| row((Slot::Priority(*one), one.label())))
            .collect(),
        false => Vec::new(),
    };
    Blocks {
        fields,
        statuses,
        priorities,
    }
}

/// How many of the source's slots are still gaps.
pub(super) fn gaps(held: &Held) -> usize {
    let blocks = mapping(held);
    let all = blocks
        .fields
        .iter()
        .chain(&blocks.statuses)
        .chain(&blocks.priorities);
    let gap = |one: &&Row| matches!(&one.value, Value::Mapped { role, .. } if *role != Role::Text);
    all.filter(gap).count()
}

/// A slot's value as the button that opens its menu, and what its gap costs beside it.
fn slotted(held: &Held, slot: Slot, label: &'static str) -> Row {
    let now = held.held(slot);
    let (shown, role, note) = match (now, slot) {
        (Some(name), _) => (name.to_string(), Role::Text, None),
        (None, Slot::Name(which @ (Mapped::Assignee | Mapped::Sprint))) => (
            "pick".to_string(),
            Role::Bad,
            Some((which.cost().to_string(), Role::Bad)),
        ),
        (None, Slot::Name(which)) => (
            "pick".to_string(),
            Role::Warn,
            Some((which.cost().to_string(), Role::Warn)),
        ),
        (None, _) => ("pick".to_string(), Role::Warn, None),
    };
    let words = match slot {
        Slot::Name(_) => "property field name",
        Slot::Status(_) => "status value",
        Slot::Priority(_) => "priority value",
        Slot::Unit => "estimate unit hours days",
    };
    let value = Value::Mapped {
        shown,
        role,
        target: Target::SettingsPick(held.id(), slot),
        note,
    };
    Row::new(Section::Providers, label, words, value)
}

/// A name takes a property of its type; a status or priority, a value of the property mapped.
fn offered(held: &Held, slot: Slot, schema: &[Property]) -> Vec<String> {
    let values_of = |which: Mapped| {
        let name = held.held(Slot::Name(which));
        let property = schema.iter().find(|one| Some(one.name.as_str()) == name);
        property.map(|one| one.options.clone()).unwrap_or_default()
    };
    match slot {
        Slot::Name(which) => {
            let fits = schema.iter().filter(|one| which.fits(one.kind));
            fits.map(|one| one.name.clone()).collect()
        }
        Slot::Status(_) => values_of(Mapped::Status),
        Slot::Priority(_) => values_of(Mapped::Priority),
        Slot::Unit => EstimateUnit::ALL.map(|one| one.label().to_string()).into(),
    }
}
