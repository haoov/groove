//! The Agent section's shared repo rows: named, being named, or let go.

use groove_controllers::AppState;
use groove_types::SharedConfig;
use groove_ui_kit::base::style::Role;

use super::super::SettingsUi;
use super::super::draft::Drafted;
use super::{Row, Section, grouped};
use crate::hit::Target;

const A: Section = Section::Agent;
const WORDS: &str = "shared team repo skills plugin";

pub(super) fn shared(app: &AppState, settings: &SettingsUi) -> Vec<Row> {
    let rows = match app.config.shared() {
        Some(held) => named(app, settings, held),
        None => unnamed(app, settings),
    };
    grouped("Shared repo", rows)
}

/// The repo the team shares, where it is followed, and what its last read said.
fn named(app: &AppState, settings: &SettingsUi, held: &SharedConfig) -> Vec<Row> {
    let repo = match (settings.unsharing, app.skill.shared.as_ref()) {
        (true, _) => {
            let act = Some(("yes, stop", Target::SettingsUnshareSure));
            let shown = "stop sharing? its copy goes".to_string();
            Row::state(A, ("repo", WORDS), (shown, Role::Warn), act)
        }
        (false, copy) => {
            let shown = copy.map_or("not read yet".into(), listed);
            let act = Some(("stop", Target::SettingsUnshare));
            Row::state(A, ("repo", WORDS), (shown, Role::Ok), act)
        }
    };
    let mut out = vec![
        repo,
        Row::text(A, "url", WORDS, &held.url),
        Row::text(A, "branch", WORDS, &held.branch),
    ];
    if let Some(why) = &app.skill.unshared {
        out.push(Row::state(
            A,
            ("read", WORDS),
            (why.clone(), Role::Bad),
            None,
        ));
    }
    out
}

/// `groove-agent · platform, review`, or the marketplace alone while it lists no plugin.
fn listed(copy: &groove_controllers::skill_service::shared::Shared) -> String {
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
        return vec![Row::state(
            A,
            ("repo", WORDS),
            ("none".into(), Role::Muted),
            act,
        )];
    };
    let cancel = Some(("cancel", Target::SettingsDraftCancel));
    let mut out = vec![Row::state(
        A,
        ("repo", WORDS),
        ("setting up".into(), Role::Working),
        cancel,
    )];
    out.extend(super::drafting::fields(draft, Section::Agent));
    out.push(sent(app));
    out
}

/// What copies the repo, and what the last copy came back with.
fn sent(app: &AppState) -> Row {
    let busy = app.skill.joining.then_some("copying…");
    super::drafting::sending(Section::Agent, busy, app.skill.unshared.as_ref())
}
