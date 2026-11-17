//! The alerts with the keys: the arrows step through the rows, Enter investigates.

use groove_controllers::{AppState, Command, alert};

use super::super::Key;
use super::panes::stepped;
use crate::Ui;

pub(super) fn in_alerts(key: Key, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let listed = crate::views::alerts::listed(app, ui);
    if key == Key::Enter {
        let (Some(parent), Some(dedup_key)) =
            (app.session.selected.clone(), ui.alerts.selected.clone())
        else {
            return Vec::new();
        };
        return vec![Command::Alert(alert::Command::Investigate {
            parent,
            dedup_key,
        })];
    }
    let at = listed
        .iter()
        .position(|one| Some(one) == ui.alerts.selected.as_ref());
    if let Some(next) = stepped(key, at, listed.len()) {
        ui.alerts.selected = listed.get(next).cloned();
    }
    Vec::new()
}
