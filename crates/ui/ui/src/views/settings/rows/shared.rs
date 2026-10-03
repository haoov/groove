//! The Agent section's shared repo rows: named, being named, or let go.

use groove_controllers::AppState;
use groove_types::SharedConfig;
use groove_ui_kit::base::style::Role;

use super::super::SettingsUi;
use super::super::draft::Drafted;
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
    out.extend(super::drafting::fields(draft, Section::Agent));
    out.push(sent(app));
    out
}

/// What copies the repo, and what the last copy came back with.
fn sent(app: &AppState) -> Row {
    let busy = app.config.joining.then_some("copying…");
    super::drafting::sending(Section::Agent, busy, app.config.unshared.as_ref())
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
    Row::new(
        Section::Agent,
        label,
        "shared team repo skills plugin",
        Value::State { shown, role, act },
    )
}
