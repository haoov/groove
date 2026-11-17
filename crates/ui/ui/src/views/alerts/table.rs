//! The alerts' grid: the order a column gives the rows, and each row's cells.

use groove_controllers::AppState;
use groove_types::{Alert, AlertChild, Timestamp};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::ago;
use groove_ui_kit::widgets::{Act, Cell, Shown, Slot, Sorted};

use super::{COLUMNS, Row};
use crate::hit::Target;

/// The rows in the picked column's order, and the header's mark for it.
pub(super) fn ordered<'a>(
    app: &AppState,
    mut all: Vec<Row<'a>>,
    sort: Option<&(String, bool)>,
) -> (Sorted, Vec<Row<'a>>) {
    let Some((label, descending)) = sort else {
        return (Sorted::NONE, all);
    };
    let Some(column) = COLUMNS.iter().position(|(one, _)| one == label) else {
        return (Sorted::NONE, all);
    };
    all.sort_by_cached_key(|row| key(app, row, column));
    if *descending {
        all.reverse();
    }
    let sorted = Sorted {
        column,
        descending: *descending,
    };
    (sorted, all)
}

/// What a row orders by in a column: the rank, the age, else its text.
fn key(app: &AppState, row: &Row, column: usize) -> (i64, String) {
    let child = match row {
        Row::Firing(_, child) => *child,
        Row::Resolved(child) => Some(*child),
    };
    let Row::Firing(one, _) = row else {
        let said = child.map(|one| said(app, one).0).unwrap_or_default();
        return (i64::MAX, said);
    };
    let text = |field: &Option<String>| field.clone().unwrap_or_default();
    match column {
        0 => (i64::from(one.rank()), String::new()),
        1 => (0, one.title.clone()),
        2 => (0, text(&one.environment)),
        3 => (0, text(&one.cluster)),
        4 => (0, text(&one.namespace)),
        5 => (-one.created_at.seconds(), String::new()),
        6 => (0, child.map(|one| said(app, one).0).unwrap_or_default()),
        _ => (i64::from(child.is_some()), String::new()),
    }
}

/// One alert across the grid, ending on its child's state and the button that opens or spawns it.
pub(super) fn shown<'a>(app: &AppState, row: &'a Row<'a>, now: Timestamp) -> Shown<'a, Target> {
    let (key, slots): (&String, Vec<Slot<'a, Target>>) = match row {
        Row::Firing(one, child) => {
            let label = if child.is_some() {
                "Open"
            } else {
                "Investigate"
            };
            let act = Act::Button(label, Target::Investigate(one.dedup_key.clone()));
            let state = match child {
                Some(child) => state(app, child),
                None => Cell::small("", Role::Ghost),
            };
            let slots = vec![
                Cell::tab(one.priority.clone().unwrap_or_default(), rank_role(one)).into(),
                Cell::body(one.title.as_str(), Role::Text).into(),
                Cell::small(one.environment.clone().unwrap_or_default(), Role::Muted).into(),
                cluster(app, &one.cluster).into(),
                Cell::small(one.namespace.clone().unwrap_or_default(), Role::Muted).into(),
                Cell::small(ago(one.created_at.age_at(now)), Role::Ghost).into(),
                state.into(),
                act.into(),
            ];
            (&one.dedup_key, slots)
        }
        Row::Resolved(child) => {
            let slots = vec![
                Cell::tab("resolved", Role::Ghost).into(),
                Cell::body(child.title.as_str(), Role::Ghost).into(),
                Cell::small("", Role::Ghost).into(),
                cluster(app, &child.cluster).into(),
                Cell::small(child.namespace.clone().unwrap_or_default(), Role::Ghost).into(),
                Cell::small("", Role::Ghost).into(),
                state(app, child).into(),
                Act::Button("Open", Target::Investigate(child.dedup_key.clone())).into(),
            ];
            (&child.dedup_key, slots)
        }
    };
    Shown::row(slots).target(Target::AlertRow(key.clone()))
}

/// The cluster in the hue Settings gave it.
fn cluster<'a>(app: &AppState, cluster: &Option<String>) -> Cell<'a> {
    let name = cluster.clone().unwrap_or_default();
    let role = app
        .config
        .cluster(&name)
        .map_or(Role::Muted, |one| Role::Hue(one.hue));
    Cell::small(name, role)
}

fn rank_role(one: &Alert) -> Role {
    match one.rank() {
        0 => Role::Bad,
        1 => Role::Attention,
        2 => Role::Warn,
        _ => Role::Faint,
    }
}

fn state<'a>(app: &AppState, child: &AlertChild) -> Cell<'a> {
    let (text, role) = said(app, child);
    Cell::small(text, role)
}

/// What the child's agent does, or `closed` once it left the rail.
fn said(app: &AppState, child: &AlertChild) -> (String, Role) {
    match app.session.get(&child.session) {
        Some(open) => crate::views::rail::state_of(app, open),
        None => ("closed".to_string(), Role::Ghost),
    }
}
