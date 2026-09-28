//! A task source's first rows: on or off, and its fields while it is being turned on.

use groove_controllers::AppState;
use groove_types::ProviderId;
use groove_ui_kit::base::style::Role;

use super::super::SettingsUi;
use super::super::draft::asks;
use super::{Row, Section, Value};
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
    let Some(draft) = settings.draft.as_ref().filter(|one| one.source == source) else {
        let act = Some(("turn on", Target::SettingsTurnOn(source)));
        return vec![row("source", "on off", "off".into(), Role::Muted, act)];
    };
    let cancel = Some(("cancel", Target::SettingsDraftCancel));
    let mut out = vec![row(
        "source",
        "on off",
        "turning on".into(),
        Role::Working,
        cancel,
    )];
    let fields = asks(source).iter().zip(&draft.fields).enumerate();
    out.extend(fields.map(|(at, (ask, field))| {
        let focused = draft.at == Some(at);
        let shown = match (focused, ask.secret, field.is_empty()) {
            (true, true, _) => field.masked(),
            (true, false, _) => field.shown(),
            (false, _, true) => ask.hint.to_string(),
            (false, true, false) => "•".repeat(field.text().chars().count()),
            (false, false, false) => field.text().to_string(),
        };
        let target = Target::SettingsDraftField(at);
        let value = Value::Input {
            shown,
            focused,
            target,
        };
        Row {
            value,
            ..row(ask.label, ask.hint, String::new(), Role::Text, None)
        }
    }));
    out.push(sent(app));
    out
}

/// On, and asking to be turned off: the second click confirms.
fn turned_on(settings: &SettingsUi, source: ProviderId) -> Row {
    match settings.leaving == Some(source) {
        true => {
            let act = Some(("yes, turn off", Target::SettingsTurnOffSure(source)));
            let shown = "turn off? its token and names go".to_string();
            row("source", "on off", shown, Role::Warn, act)
        }
        false => {
            let act = Some(("turn off", Target::SettingsTurnOff(source)));
            row("source", "on off", "on".into(), Role::Ok, act)
        }
    }
}

/// What sends the fields, and what the last send came back with.
fn sent(app: &AppState) -> Row {
    let (shown, role) = match (&app.config.connecting, &app.config.refused) {
        (Some(_), _) => ("connecting…".to_string(), Role::Working),
        (None, Some(why)) => (why.clone(), Role::Bad),
        (None, None) => (String::new(), Role::Muted),
    };
    let act = app
        .config
        .connecting
        .is_none()
        .then_some(("connect", Target::SettingsConnect));
    row("", "connect", shown, role, act)
}

fn row(
    label: &'static str,
    words: &'static str,
    shown: String,
    role: Role,
    act: Option<(&'static str, Target)>,
) -> Row {
    Row {
        section: Section::Providers,
        group: "",
        label,
        words,
        value: Value::State { shown, role, act },
    }
}
