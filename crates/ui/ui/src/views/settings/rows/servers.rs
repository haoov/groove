//! The Language servers section: each server the config names, its switch, and whether it runs.

use groove_controllers::AppState;
use groove_controllers::config_service::Preference;
use groove_types::{Config, LanguageServer};
use groove_ui_kit::base::style::Role;

use super::{Row, Section, Value, grouped};
use crate::hit::Target;

pub(super) fn servers(app: &AppState) -> Vec<Row> {
    let config = app.config.config.as_ref();
    let servers = config.map_or_else(LanguageServer::defaults, Config::servers);
    let rows = servers.iter().map(|one| server(app, one)).collect();
    grouped("Servers", rows)
}

/// One server's row: its switch, then what it serves, its program and what the check found.
fn server(app: &AppState, one: &LanguageServer) -> Row {
    let on = !one.off;
    let found = app
        .config
        .servers
        .iter()
        .find(|(program, _)| *program == one.program);
    let command = std::iter::once(one.program.as_str())
        .chain(one.args.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ");
    let serves = one.languages.join(", ");
    let (state, role) = match found {
        _ if !on => ("off".to_string(), Role::Faint),
        None => ("not checked yet".to_string(), Role::Faint),
        Some((_, Some(version))) => (version.clone(), Role::Ok),
        Some((_, None)) => ("not found on the login PATH".to_string(), Role::Warn),
    };
    let value = Value::Source {
        on,
        target: Target::SetPreference(Preference::Server(one.name.clone(), !on)),
        said: Some((format!("{serves} · {command} · {state}"), role)),
        act: None,
    };
    let words = "language server lsp diagnostics completion python yaml";
    Row::new(Section::Servers, one.name.clone(), words, value)
}
