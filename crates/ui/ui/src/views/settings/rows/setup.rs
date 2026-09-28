//! The Setup section's rows: the three paths, then what the environment check found.

use std::path::Path;

use groove_controllers::AppState;
use groove_types::Tool;
use groove_ui_kit::base::style::Role;

use super::{Row, Section, Value};
use crate::hit::Target;

pub(super) fn setup(app: &AppState) -> Vec<Row> {
    let env = &app.env;
    let root = app.config.worktree_root(&env.home);
    let mut out = vec![
        path(
            "config",
            "file path json",
            shown(&env.config_dir.join("config.json"), &env.home),
        ),
        path(
            "state",
            "database sqlite path",
            shown(&env.data_dir.join("app.db"), &env.home),
        ),
        path(
            "worktree root",
            "path pool clones git",
            shown(&root, &env.home),
        ),
        checked(app),
    ];
    let tools = app.config.tools.iter().flatten();
    out.extend(tools.map(|one| tool(one, app.agent.login.is_some())));
    out
}

fn path(label: &'static str, words: &'static str, text: String) -> Row {
    let value = Value::Text { text, mono: true };
    Row {
        section: Section::Setup,
        label,
        words,
        value,
    }
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
    Row {
        section: Section::Setup,
        label: "environment",
        words: "check tools programs",
        value: Value::State { shown, role, act },
    }
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
    Row {
        section: Section::Setup,
        label: one.name,
        words: one.purpose,
        value: Value::State { shown, role, act },
    }
}

/// A path under home as `~/…`.
fn shown(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}
