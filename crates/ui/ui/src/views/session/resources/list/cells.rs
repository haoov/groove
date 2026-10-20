//! One object's cells in the list, each in the colour of what it says, and the list's columns.

use groove_controllers::AppState;
use groove_types::{Health, ObjectRow, WatchKey};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::widgets::{Cell, Column, Shown, Width};

use super::plan::{Plan, Reads};
use crate::hit::Target;
use crate::views::session::resources::health::colour;

/// One object's row: its cluster and namespace when the list spans several, then its cells.
pub(super) fn one<'a>(
    app: &AppState,
    (key, row): (&'a WatchKey, &'a ObjectRow),
    (plan, names): (&Plan, &[&str]),
) -> Shown<'a, Target> {
    let mut cells = Vec::new();
    if plan.clusters {
        let hue = app.config.cluster(&key.context).map(|one| one.hue);
        cells.push(Cell::small(
            key.context.as_str(),
            hue.map_or(Role::Faint, Role::Hue),
        ));
    }
    if plan.namespaces {
        cells.push(Cell::small(
            row.namespace.as_deref().unwrap_or_default(),
            Role::Muted,
        ));
    }
    let cell = |at: usize| row.cells.get(at).map(String::as_str).unwrap_or_default();
    let ended = plan.shown.iter().zip(&plan.reads).any(|(at, reads)| {
        *reads == Reads::Status && groove_types::status_health(cell(*at)) == Some(Health::Done)
    });
    for (place, (at, reads)) in plan.shown.iter().zip(&plan.reads).enumerate() {
        let text = cell(*at);
        let health = match reads {
            Reads::Status => groove_types::status_health(text),
            Reads::Ready if ended => Some(Health::Done),
            Reads::Ready => groove_types::ready_health(text),
            Reads::Plain | Reads::Time => None,
        };
        let role = match (place, health) {
            (0, _) => Role::Text,
            (_, Some(health)) => colour(health),
            _ => Role::Muted,
        };
        let cell = match reads {
            Reads::Time => Cell::small(text, role),
            _ => Cell::body(text, role),
        };
        cells.push(match place {
            0 => cell.lit(groove_types::spans(text, names)),
            _ => cell,
        });
    }
    Shown::row(cells).target(Target::ResourceRow(row.uid.clone()))
}

/// A column the list sorts by its label and drags by its edge.
pub(super) fn column<'a>(label: &'a str, width: Width<'a>) -> Column<'a, Target> {
    Column {
        label,
        width,
        end: false,
        sort: Some(Target::ResourceSort(label.to_string())),
        edge: Some(Target::ResourceEdge(label.to_string())),
    }
}
