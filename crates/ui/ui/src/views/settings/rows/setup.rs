//! The Setup section's rows: what the environment check found, the sign-in, the paths, what's next.

use std::path::Path;

use groove_controllers::AppState;
use groove_types::Tool;
use groove_ui_kit::base::style::Role;

use super::{Row, Section, Value, grouped};
use crate::hit::Target;

pub(super) fn setup(app: &AppState) -> Vec<Row> {
    let signing_in = app.config.login.is_some() || app.config.signing_in;
    let (login, tools): (Vec<&Tool>, Vec<&Tool>) = app
        .config
        .tools
        .iter()
        .flatten()
        .partition(|one| one.name == "claude");
    let checks =
        std::iter::once(checked(app)).chain(tools.into_iter().map(|one| tool(one, signing_in)));
    let mut out = grouped("Environment", checks.collect());
    let login = login.into_iter().map(|one| tool(one, signing_in));
    out.extend(grouped("Claude login", login.collect()));
    out.extend(grouped("Files", paths(app)));
    if no_source(app) {
        out.extend(grouped("Next", vec![providers()]));
    }
    out
}

/// The config file, the database, and the root the worktrees stand under.
fn paths(app: &AppState) -> Vec<Row> {
    let env = &app.env;
    let root = app.config.worktree_root(&env.home);
    vec![
        Row::text(
            Section::Setup,
            "config",
            "file path json",
            shown(&env.config_file(), &env.home),
        ),
        Row::text(
            Section::Setup,
            "state",
            "database sqlite path",
            shown(&env.database(), &env.home),
        ),
        Row::text(
            Section::Setup,
            "worktree root",
            "path pool clones git",
            shown(&root, &env.home),
        ),
    ]
}

/// The way to Providers while no task source is set up.
fn providers() -> Row {
    let next = Value::State {
        shown: "no task source · Notion or GitHub".into(),
        role: Role::Attention,
        act: Some(("open", Target::SettingsSection(Section::Providers))),
    };
    Row::new(
        Section::Setup,
        "providers",
        "task source notion github",
        next,
    )
}

/// What Setup still needs: a required program missing, claude signed out, no config file.
pub fn left(app: &AppState) -> usize {
    let tools = app.config.tools.iter().flatten();
    let short = tools
        .filter(|one| match &one.found {
            None => one.required,
            Some(found) => found.signed_in == Some(false) && one.name == "claude",
        })
        .count();
    short + usize::from(app.config.config.is_none())
}

/// The gaps of every source set up.
pub fn gaps(app: &AppState) -> usize {
    let config = app.config.config.as_ref();
    let notion = config
        .and_then(|c| c.notion.as_ref())
        .map(super::mapping::Held::Notion);
    let github = config
        .and_then(|c| c.github.as_ref())
        .map(super::mapping::Held::Github);
    notion
        .into_iter()
        .chain(github)
        .map(|one| super::mapping::gaps(&one))
        .sum()
}

/// No task source is set up yet.
pub fn no_source(app: &AppState) -> bool {
    groove_controllers::task_service::source_ids(app.config.config.as_ref()).is_empty()
}

/// The check as a whole: running, every program ready, or how many are not.
fn checked(app: &AppState) -> Row {
    let short = app.config.tools.iter().flatten().filter(|one| !one.ready());
    let (shown, role) = match (app.config.checking, short.count()) {
        (true, _) => ("checking…".to_string(), Role::Working),
        (false, 0) if app.config.tools.is_some() => ("all ready".to_string(), Role::Ok),
        (false, 0) => ("not checked yet".to_string(), Role::Muted),
        (false, n) => (format!("{n} not ready"), Role::Warn),
    };
    let act = (!app.config.checking).then_some(("check again", Target::SettingsCheck));
    Row::new(
        Section::Setup,
        "check",
        "environment tools programs",
        Value::State { shown, role, act },
    )
}

/// One program: its version and sign-in, or what is lost without it.
fn tool(one: &Tool, signing_in: bool) -> Row {
    let (shown, role) = match &one.found {
        None if one.required => (format!("not found · {}", one.purpose), Role::Bad),
        None => (format!("not found · {}", one.purpose), Role::Warn),
        Some(found) => match found.signed_in {
            Some(true) => (format!("{} · signed in", found.version), Role::Ok),
            Some(false) => (format!("{} · not signed in", found.version), Role::Warn),
            None => (found.version.clone(), Role::Ok),
        },
    };
    let act = match (one.name, signing_in) {
        ("claude", false) if one.found.is_some() => Some(("sign in", Target::SettingsLogin)),
        ("claude", true) => Some(("cancel", Target::SettingsLoginEnd)),
        _ => None,
    };
    Row::new(
        Section::Setup,
        one.name,
        one.purpose,
        Value::State { shown, role, act },
    )
}

/// A path under home as `~/…`.
fn shown(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}
