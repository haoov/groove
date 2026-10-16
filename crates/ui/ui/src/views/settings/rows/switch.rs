//! A task source's first rows: on or off, and its fields while it is being turned on.

use groove_controllers::AppState;
use groove_types::ProviderId;
use groove_ui_kit::base::style::Role;

use super::super::SettingsUi;
use super::super::draft::Drafted;
use super::{Row, Section, Value};

const P: Section = Section::Providers;
use crate::hit::Target;

pub(super) fn switch(
    app: &AppState,
    settings: &SettingsUi,
    source: ProviderId,
    gaps: Option<usize>,
) -> Vec<Row> {
    if let Some(gaps) = gaps {
        return vec![turned_on(settings, source, gaps)];
    }
    let drafted = Drafted::Source(source);
    let Some(draft) = settings.draft.as_ref().filter(|one| one.source == drafted) else {
        return vec![toggled(false, Target::SettingsTurnOn(source), None, None)];
    };
    let said = Some(("turning on".to_string(), Role::Working));
    let mut out = vec![toggled(true, Target::SettingsDraftCancel, said, None)];
    out.extend(super::drafting::fields(draft, Section::Providers));
    out.push(sent(app));
    out
}

/// On, and asking to be turned off: the second click confirms.
fn turned_on(settings: &SettingsUi, source: ProviderId, gaps: usize) -> Row {
    let off = Target::SettingsTurnOff(source);
    match settings.leaving == Some(source) {
        true => {
            let said = Some(("its token and names go".to_string(), Role::Warn));
            let act = Some(("yes, turn off", Target::SettingsTurnOffSure(source)));
            toggled(true, off, said, act)
        }
        false => {
            let said = match gaps {
                0 => ("connected".to_string(), Role::Ok),
                1 => ("1 gap".to_string(), Role::Warn),
                n => (format!("{n} gaps"), Role::Warn),
            };
            toggled(true, off, Some(said), None)
        }
    }
}

/// The source's row: its toggle, what it says beside it, and the action that answers.
fn toggled(
    on: bool,
    target: Target,
    said: Option<(String, Role)>,
    act: Option<(&'static str, Target)>,
) -> Row {
    let value = Value::Source {
        on,
        target,
        said,
        act,
    };
    Row::new(P, "source", "on off", value)
}

/// What sends the fields, and what the last send came back with.
fn sent(app: &AppState) -> Row {
    let busy = app.config.connecting.map(|_| "connecting…");
    super::drafting::sending(Section::Providers, busy, app.config.refused.as_ref())
}
