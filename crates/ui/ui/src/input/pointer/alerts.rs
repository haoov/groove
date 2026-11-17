//! A click on the alerts' window: its header folds it, a row opens the alert's child.

use groove_controllers::{AppState, Command, alert};

use crate::Ui;
use crate::hit::Target;

pub(super) fn acted(target: &Target, ui: &mut Ui, app: &AppState) -> Option<Vec<Command>> {
    match target {
        Target::AlertsFold => {
            ui.alerts.open = !ui.alerts.open;
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
