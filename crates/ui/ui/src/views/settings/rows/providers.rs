//! The Providers section's rows: each task source under its heading, then the forge tokens.

use groove_controllers::AppState;
use groove_types::{Config, PropertyNames};
use groove_ui_kit::base::style::Role;

use super::{Row, Section, Value, grouped};

pub(super) fn providers(app: &AppState) -> Vec<Row> {
    let config = app.config.config.as_ref();
    let mut out = grouped("Notion", notion(config));
    out.extend(grouped("GitHub", github(config)));
    out.extend(grouped("Forge tokens", forges(app)));
    out
}

fn notion(config: Option<&Config>) -> Vec<Row> {
    let Some(notion) = config.and_then(|c| c.notion.as_ref()) else {
        return vec![source(false)];
    };
    let mut out = vec![
        source(true),
        field("database", "database id", &notion.database_id),
        field("user", "user id assignee", &notion.user_id),
        state("token", "token secret", "held".into(), Role::Ok),
    ];
    out.extend(mapped(&notion.properties));
    out
}

fn github(config: Option<&Config>) -> Vec<Row> {
    let Some(github) = config.and_then(|c| c.github.as_ref()) else {
        return vec![source(false)];
    };
    let token = match github.token {
        Some(_) => "held",
        None => "from gh",
    };
    let mut out = vec![
        source(true),
        field("host", "host issues", &github.host),
        state("token", "token secret", token.into(), Role::Ok),
    ];
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

fn source(on: bool) -> Row {
    let (shown, role) = match on {
        true => ("on", Role::Ok),
        false => ("off", Role::Muted),
    };
    state("source", "tasks provider", shown.into(), role)
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
