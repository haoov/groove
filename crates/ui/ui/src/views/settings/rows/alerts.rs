//! The Alerts section's rows: the `inc` login the alerts read with, and how often they read.

use groove_controllers::AppState;
use groove_controllers::config_service::{Preference, SignIn};
use groove_ui_kit::base::style::Role;

use super::preferences::count;
use super::{Row, Section, grouped};
use crate::hit::Target;

pub(super) fn alerts(app: &AppState) -> Vec<Row> {
    let poll = count(
        "poll interval",
        "incident.io inc alerts maintenance poll",
        (app.config.preferences().alerts_poll_secs, 10, 10),
        "seconds",
        Preference::AlertsPollSecs,
    );
    let rows = vec![
        login(app),
        Row {
            section: Section::Alerts,
            ..poll
        },
    ];
    grouped("incident.io", rows)
}

/// What the last read of the alerts said of the login, and the login or its cancel.
fn login(app: &AppState) -> Row {
    let config = &app.config;
    let running = (config.login.is_some() || config.signing_in) && config.signing == SignIn::Inc;
    let (shown, role) = match (&app.alert.failed, app.alert.read_at) {
        _ if running => ("logging in…".to_string(), Role::Working),
        (Some(failed), _) => (failed.clone(), Role::Warn),
        (None, Some(_)) => ("logged in".to_string(), Role::Ok),
        (None, None) => ("not read yet".to_string(), Role::Muted),
    };
    let act = match (running, &app.alert.failed, app.alert.read_at) {
        (true, ..) => ("cancel", Target::SettingsLoginEnd),
        (false, None, Some(_)) => ("log in again", Target::SettingsIncLogin),
        _ => ("log in", Target::SettingsIncLogin),
    };
    let words = "inc incident.io login token alerts";
    Row::state(Section::Alerts, ("login", words), (shown, role), Some(act))
}
