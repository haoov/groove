//! The Providers section's rows: each task source under its heading, then the forge tokens.

use groove_controllers::AppState;
use groove_types::ProviderId;
use groove_ui_kit::base::style::Role;

use super::super::SettingsUi;
use super::mapping::{Held, mapping};
use super::switch::switch;
use super::{Row, Section, grouped};

const P: Section = Section::Providers;

pub(super) fn providers(app: &AppState, settings: &SettingsUi) -> Vec<Row> {
    let mut out = grouped("Notion", notion(app, settings));
    out.extend(grouped("GitHub", github(app, settings)));
    out.extend(grouped("Forge tokens", forges(app)));
    out
}

fn notion(app: &AppState, settings: &SettingsUi) -> Vec<Row> {
    let held = app.config.config.as_ref().and_then(|c| c.notion.as_ref());
    let mut out = switch(app, settings, ProviderId::Notion, held.is_some());
    let Some(notion) = held else {
        return out;
    };
    out.extend([
        Row::text(P, "database", "database id", &notion.database_id),
        Row::text(P, "user", "user id assignee", &notion.user_id),
        Row::state(
            P,
            ("token", "token secret"),
            ("held".into(), Role::Ok),
            None,
        ),
    ]);
    out.extend(mapping(&Held::Notion(notion)));
    out
}

fn github(app: &AppState, settings: &SettingsUi) -> Vec<Row> {
    let held = app.config.config.as_ref().and_then(|c| c.github.as_ref());
    let mut out = switch(app, settings, ProviderId::Github, held.is_some());
    let Some(github) = held else {
        return out;
    };
    let token = match github.token {
        Some(_) => "held",
        None => "from gh",
    };
    out.extend([
        Row::text(P, "host", "host issues", &github.host),
        Row::state(P, ("token", "token secret"), (token.into(), Role::Ok), None),
    ]);
    out.extend(mapping(&Held::Github(github)));
    out
}

/// The forges' own tokens, as the environment check found their sign-in.
fn forges(app: &AppState) -> Vec<Row> {
    let signed_in = |name: &str| {
        let tools = app.config.tools.iter().flatten();
        let found = tools
            .filter(|one| one.name == name)
            .find_map(|one| one.found.as_ref());
        found.map(|found| found.signed_in == Some(true))
    };
    let row = |label: &'static str| {
        let (shown, role) = match signed_in(label) {
            Some(true) => ("present", Role::Ok),
            Some(false) => ("missing", Role::Warn),
            None if app.config.tools.is_some() => ("no cli", Role::Warn),
            None => ("not checked yet", Role::Muted),
        };
        Row::state(P, (label, "token cli"), (shown.into(), role), None)
    };
    vec![row("gh"), row("glab")]
}
