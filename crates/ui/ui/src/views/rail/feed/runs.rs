//! Back-to-back events of one kind from one session, folded into one row.

use groove_controllers::Told;
use groove_types::{Error, SessionId, TimelineEvent, TimelineKind, Timestamp};

use super::super::{Fold, RunKey};

/// One thing that happened.
#[derive(Clone, Copy)]
pub(super) enum Item<'a> {
    Bad(&'a Told<Error>),
    Said(&'a Told<String>),
    Event(&'a TimelineEvent),
}

impl Item<'_> {
    pub(super) fn at(&self) -> Timestamp {
        match self {
            Item::Bad(one) => one.at,
            Item::Said(one) => one.at,
            Item::Event(one) => one.at,
        }
    }

    pub(super) fn session(&self) -> Option<&SessionId> {
        match self {
            Item::Event(one) => Some(&one.session),
            _ => None,
        }
    }

    fn fold(&self) -> Fold {
        match self {
            Item::Bad(_) => Fold::Error,
            Item::Said(_) => Fold::Note,
            Item::Event(one) if one.kind == TimelineKind::TurnEnd => {
                Fold::Event(TimelineKind::TurnStart)
            }
            Item::Event(one) => Fold::Event(one.kind),
        }
    }
}

/// A run of items, newest first.
#[derive(Clone)]
pub(super) struct Run<'a> {
    pub items: Vec<Item<'a>>,
}

impl Run<'_> {
    pub(super) fn key(&self) -> RunKey {
        let newest = &self.items[0];
        RunKey {
            fold: newest.fold(),
            newest: newest.at(),
        }
    }
}

/// The items, newest first, each run of one kind from one session in one row.
pub(super) fn folded(items: Vec<Item<'_>>) -> Vec<Run<'_>> {
    let mut out: Vec<Run<'_>> = Vec::new();
    for item in items {
        match out.last_mut() {
            Some(run) if same(&run.items[0], &item) => run.items.push(item),
            _ => out.push(Run { items: vec![item] }),
        }
    }
    out
}

fn same(a: &Item<'_>, b: &Item<'_>) -> bool {
    a.fold() == b.fold() && a.session() == b.session()
}
