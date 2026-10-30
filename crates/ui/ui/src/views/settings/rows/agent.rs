//! The Agent section's rows: how new sessions approve, how the agent shows, then the skills they are given.

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
    let drawn = Value::Toggle {
        on: held.conversation,
        flip: Preference::Conversation(!held.conversation),
    };
    let conversation = Row::new(
        Section::Agent,
        "conversation view",
        "headless, from its next start",
        drawn,
    );
    let mut out = grouped("Approvals", vec![approve]);
    out.extend(grouped("View", vec![conversation]));
    out.extend(shared(app, settings));
    out.extend(super::skills::skills(app, settings));
    out.extend(super::routines::routines(app));
    out
}
