//! A click on the alerts' window: fold, order, select, or open a child.

use groove_controllers::{AppState, Command, alert};

use crate::Ui;
use crate::hit::{Hits, Target};
use crate::views::session::resources::Columns;

pub(super) fn acted(target: &Target, ui: &mut Ui, app: &AppState) -> Option<Vec<Command>> {
    match target {
        Target::AlertsFold => {
            ui.alerts.open = !ui.alerts.open;
            Some(Vec::new())
        }
        Target::AlertRow(key) => {
            ui.alerts.selected = Some(key.clone());
            Some(Vec::new())
        }
        Target::AlertColumn(label) => {
            ui.alerts.sort = match ui.alerts.sort.take() {
                Some((at, descending)) if at == *label => Some((at, !descending)),
                _ => Some((label.clone(), false)),
            };
            Some(Vec::new())
        }
        Target::Investigate(key) => {
            let Some(parent) = app.session.selected.clone() else {
                return Some(Vec::new());
            };
            let investigate = alert::Command::Investigate {
                parent,
                dedup_key: key.clone(),
            };
            Some(vec![Command::Alert(investigate)])
        }
        _ => None,
    }
}

/// A press on an alerts column's edge holds it, at the width its header was drawn.
pub(super) fn held(ui: &mut Ui, hits: &Hits, (label, x): (String, f32)) -> Vec<Command> {
    if let Some(header) = hits.rect_of(&Target::AlertColumn(label.clone())) {
        super::resources::hold(ui, Columns::Alerts, label, (x, header.w));
    }
    Vec::new()
}
