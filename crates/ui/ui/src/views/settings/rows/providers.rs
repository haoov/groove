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
    let config = app.config.config.as_ref();
    let notion = config.and_then(|c| c.notion.as_ref()).map(Held::Notion);
    let github = config.and_then(|c| c.github.as_ref()).map(Held::Github);
    let mut out = source(app, settings, (ProviderId::Notion, notion), NOTION);
    out.extend(source(app, settings, (ProviderId::Github, github), GITHUB));
    out.extend(grouped("Forge tokens", forges(app)));
    out
}

/// A source's four headings: the source, its fields, its status values, its priority values.
type Headings = [&'static str; 4];
const NOTION: Headings = [
    "Notion",
    "Notion fields",
    "Notion status values",
    "Notion priority values",
];
const GITHUB: Headings = [
    "GitHub",
    "GitHub fields",
    "GitHub status values",
    "GitHub priority values",
];

/// The source's toggle and connection, then its mapping block by block.
fn source(
    app: &AppState,
    settings: &SettingsUi,
    (id, held): (ProviderId, Option<Held>),
    heads: Headings,
) -> Vec<Row> {
    let gaps = held.as_ref().map(super::mapping::gaps);
    let mut out = switch(app, settings, id, gaps);
    let Some(held) = held else {
        return grouped(heads[0], out);
    };
    out.extend(connection(&held));
    let mut out = grouped(heads[0], out);
    let blocks = mapping(&held);
    out.extend(grouped(heads[1], blocks.fields));
    out.extend(grouped(heads[2], blocks.statuses));
    out.extend(grouped(heads[3], blocks.priorities));
    out
}

/// Where the source is read from, and with what.
fn connection(held: &Held) -> Vec<Row> {
    let held_token = ("held".to_string(), Role::Ok);
    match held {
        Held::Notion(notion) => vec![
            Row::text(P, "database", "database id", &notion.database_id),
            Row::text(P, "user", "user id assignee", &notion.user_id),
            Row::state(P, ("token", "token secret"), held_token, None),
        ],
        Held::Github(github) => {
            let token = match github.token {
                Some(_) => held_token,
                None => ("from gh".to_string(), Role::Ok),
            };
            vec![
                Row::text(P, "host", "host issues", &github.host),
                Row::state(P, ("token", "token secret"), token, None),
            ]
        }
    }
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
