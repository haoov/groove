//! The Agent section's shared repo rows: named, being named, or let go.

use groove_controllers::AppState;
use groove_types::SharedConfig;
use groove_ui_kit::base::style::Role;

use super::super::SettingsUi;
use super::super::draft::{Drafted, asks};
use super::{Row, Section, Value, grouped};
use crate::hit::Target;

pub(super) fn shared(app: &AppState, settings: &SettingsUi) -> Vec<Row> {
    let rows = match app.config.shared() {
        Some(held) => named(app, settings, held),
        None => unnamed(app, settings),
    };
    grouped("Shared repo", rows)
}

/// The repo the team shares, where it is followed, and what its last read said.
fn named(app: &AppState, settings: &SettingsUi, held: &SharedConfig) -> Vec<Row> {
    let repo = match (settings.unsharing, app.agent.shared.as_ref()) {
        (true, _) => {
            let act = Some(("yes, stop", Target::SettingsUnshareSure));
            state(
                "repo",
                "stop sharing? its copy goes".into(),
                Role::Warn,
                act,
            )
        }
        (false, copy) => {
            let shown = copy.map_or("not read yet".into(), listed);
            state(
                "repo",
                shown,
                Role::Ok,
                Some(("stop", Target::SettingsUnshare)),
            )
        }
    };
    let mut out = vec![repo, text("url", &held.url), text("branch", &held.branch)];
    if let Some(why) = &app.config.unshared {
        out.push(state("read", why.clone(), Role::Bad, None));
    }
    out
}

/// `groove-agent · platform, review`, or the marketplace alone while it lists no plugin.
fn listed(copy: &groove_controllers::agent_service::shared::Shared) -> String {
    let market = &copy.marketplace;
    let plugins: Vec<&str> = market.plugins.iter().map(|one| one.name.as_str()).collect();
    match plugins.is_empty() {
        true => format!("{} · no plugin", market.name),
        false => format!("{} · {}", market.name, plugins.join(", ")),
    }
}

/// No repo yet: what names one, and its fields while they are typed.
fn unnamed(app: &AppState, settings: &SettingsUi) -> Vec<Row> {
    let Some(draft) = settings
        .draft
        .as_ref()
        .filter(|one| one.source == Drafted::Shared)
    else {
        let act = Some(("set", Target::SettingsShare));
        return vec![state("repo", "none".into(), Role::Muted, act)];
    };
    let cancel = Some(("cancel", Target::SettingsDraftCancel));
    let mut out = vec![state("repo", "setting up".into(), Role::Working, cancel)];
    let fields = asks(Drafted::Shared).iter().zip(&draft.fields).enumerate();
    out.extend(fields.map(|(at, (ask, field))| {
        let focused = draft.at == Some(at);
        let shown = match (focused, field.is_empty()) {
            (true, _) => field.shown(),
            (false, true) => ask.hint.to_string(),
            (false, false) => field.text().to_string(),
        };
        let target = Target::SettingsDraftField(at);
        let value = Value::Input {
            shown,
            focused,
            target,
        };
        Row {
            value,
            ..state(ask.label, String::new(), Role::Text, None)
        }
    }));
    out.push(sent(app));
    out
}

/// What copies the repo, and what the last copy came back with.
fn sent(app: &AppState) -> Row {
    let (shown, role) = match (app.config.joining, &app.config.unshared) {
        (true, _) => ("copying…".to_string(), Role::Working),
        (false, Some(why)) => (why.clone(), Role::Bad),
        (false, None) => (String::new(), Role::Muted),
    };
    let act = (!app.config.joining).then_some(("connect", Target::SettingsConnect));
    state("", shown, role, act)
}

fn text(label: &'static str, text: &str) -> Row {
    let value = Value::Text {
        text: text.to_string(),
        mono: true,
    };
    Row {
        value,
        ..state(label, String::new(), Role::Text, None)
    }
}

fn state(
    label: &'static str,
    shown: String,
    role: Role,
    act: Option<(&'static str, Target)>,
) -> Row {
    Row {
        section: Section::Agent,
        group: "",
        label,
        words: "shared team repo skills plugin",
        value: Value::State { shown, role, act },
    }
}
