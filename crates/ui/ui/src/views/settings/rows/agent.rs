//! The Agent section's rows: how new sessions approve, then the skills they are given.

use groove_controllers::AppState;
use groove_controllers::config_service::Preference;

use super::super::SettingsUi;
use super::shared::shared;
use super::{Row, Section, Value, grouped};

pub(super) fn agent(app: &AppState, settings: &SettingsUi) -> Vec<Row> {
    let held = app.config.preferences();
    let toggle = Value::Toggle {
        on: held.auto_approve_default,
        flip: Preference::AutoApproveDefault(!held.auto_approve_default),
    };
    let approve = Row::new(
        Section::Agent,
        "auto-approve default",
        "writes ask new sessions",
        toggle,
    );
    let mut out = grouped("Approvals", vec![approve]);
    out.extend(shared(app, settings));
    out.extend(super::skills::skills(app, settings));
    out.extend(super::routines::routines(app));
    out
}
