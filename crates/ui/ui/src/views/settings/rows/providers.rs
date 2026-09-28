//! The Providers section's rows: each task source under its heading, then the forge tokens.

use groove_controllers::AppState;
use groove_types::{PropertyNames, ProviderId};
use groove_ui_kit::base::style::Role;

use super::super::SettingsUi;
use super::switch::switch;
use super::{Row, Section, Value, grouped};

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
        field("database", "database id", &notion.database_id),
        field("user", "user id assignee", &notion.user_id),
        state("token", "token secret", "held".into(), Role::Ok),
        required("assignee", "people property yours", notion.assignee()),
        required("sprint", "relation running current", notion.sprint()),
    ]);
    out.extend(mapped(&notion.properties));
    out
}

/// A name the source reads no task without.
fn required(label: &'static str, words: &'static str, name: Option<&str>) -> Row {
    match name {
        Some(name) => state(label, words, name.to_string(), Role::Text),
        None => state(label, words, "gap · no task is listed".into(), Role::Bad),
    }
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
        field("host", "host issues", &github.host),
        state("token", "token secret", token.into(), Role::Ok),
    ]);
    out.extend(mapped(&github.properties));
    out
}

/// The six properties Groove reads, each with the source's own name or what its gap costs.
fn mapped(names: &PropertyNames) -> Vec<Row> {
    let status = Some(names.status.clone()).filter(|name| !name.is_empty());
    let six = [
        ("status", status, "a task cannot move"),
        (
            "priority",
            names.priority.clone(),
            "no priority on the board",
        ),
        ("start", names.start.clone(), "no start date on the plan"),
        ("due", names.due.clone(), "no due date, no due-soon warning"),
        (
            "estimate",
            names.estimate.clone(),
            "no estimate beside the time",
        ),
        (
            "logged",
            names.logged.clone(),
            "the hours measured are not logged",
        ),
    ];
    let row = |(label, name, cost): (&'static str, Option<String>, &str)| {
        let (shown, role) = match name {
            Some(name) => (name, Role::Text),
            None => (format!("gap · {cost}"), Role::Warn),
        };
        state(label, "property field name", shown, role)
    };
    six.into_iter().map(row).collect()
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
        state(label, "token cli", shown.into(), role)
    };
    vec![row("gh"), row("glab")]
}

fn field(label: &'static str, words: &'static str, text: &str) -> Row {
    let value = Value::Text {
        text: text.to_string(),
        mono: true,
    };
    Row {
        section: Section::Providers,
        group: "",
        label,
        words,
        value,
    }
}

fn state(label: &'static str, words: &'static str, shown: String, role: Role) -> Row {
    let act = None;
    Row {
        section: Section::Providers,
        group: "",
        label,
        words,
        value: Value::State { shown, role, act },
    }
}
