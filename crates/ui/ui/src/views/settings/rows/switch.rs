//! A task source's first rows: on or off, and its fields while it is being turned on.

use groove_controllers::AppState;
use groove_types::ProviderId;
use groove_ui_kit::base::style::Role;

use super::super::SettingsUi;
use super::super::draft::Drafted;
use super::{Row, Section};

const P: Section = Section::Providers;
use crate::hit::Target;

pub(super) fn switch(
    app: &AppState,
    settings: &SettingsUi,
    source: ProviderId,
    on: bool,
) -> Vec<Row> {
    if on {
        return vec![turned_on(settings, source)];
    }
    let drafted = Drafted::Source(source);
    let Some(draft) = settings.draft.as_ref().filter(|one| one.source == drafted) else {
        let act = Some(("turn on", Target::SettingsTurnOn(source)));
        return vec![Row::state(
            P,
            ("source", "on off"),
            ("off".into(), Role::Muted),
            act,
        )];
    };
    let cancel = Some(("cancel", Target::SettingsDraftCancel));
    let mut out = vec![Row::state(
        P,
        ("source", "on off"),
        ("turning on".into(), Role::Working),
        cancel,
    )];
    out.extend(super::drafting::fields(draft, Section::Providers));
    out.push(sent(app));
    out
}

/// On, and asking to be turned off: the second click confirms.
fn turned_on(settings: &SettingsUi, source: ProviderId) -> Row {
    match settings.leaving == Some(source) {
        true => {
            let act = Some(("yes, turn off", Target::SettingsTurnOffSure(source)));
            let shown = "turn off? its token and names go".to_string();
            Row::state(P, ("source", "on off"), (shown, Role::Warn), act)
        }
        false => {
            let act = Some(("turn off", Target::SettingsTurnOff(source)));
            Row::state(P, ("source", "on off"), ("on".into(), Role::Ok), act)
        }
    }
}

/// What sends the fields, and what the last send came back with.
fn sent(app: &AppState) -> Row {
    let busy = app.config.connecting.map(|_| "connecting…");
    super::drafting::sending(Section::Providers, busy, app.config.refused.as_ref())
}
