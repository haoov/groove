//! The `config` controller: one function per user action on the `config` service.

mod environment;
mod mapping;
mod sources;

use std::collections::BTreeMap;

use groove_config_service::Preference;
use groove_types::{Mapping, ProviderId, Secret};

use crate::{AppState, Services, Spawner};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `config.set_preference`: one preference changed and the file written.
    SetPreference(Preference),
    /// `config.check_environment`: each program's version and sign-in, read again.
    CheckEnvironment,
    /// `config.login`: `claude auth login` on a terminal of Setup's own.
    Login {
        cols: u16,
        rows: u16,
    },
    /// What goes to the sign-in's terminal while it runs.
    SendLogin {
        bytes: Vec<u8>,
    },
    PasteLogin {
        text: String,
    },
    ResizeLogin {
        cols: u16,
        rows: u16,
    },
    /// The sign-in ended before it was done.
    EndLogin,
    /// `config.set_task_source`: Notion on, once the token reads the database.
    ConnectNotion {
        token: Secret,
        database_id: String,
        user_id: String,
    },
    /// GitHub on, once the host answers the token `gh` holds for it.
    ConnectGithub {
        host: String,
    },
    /// A source off, its block gone; the last one stays.
    TurnOff(ProviderId),
    /// `config.read_schema`: the properties a source holds, for the mapping.
    ReadSchema(ProviderId),
    /// `config.map`: one name or value of a source mapped.
    Map {
        source: ProviderId,
        change: Mapping,
    },
    /// `config.rebind`: the chords that replace the keymap's defaults, whole.
    Rebind(BTreeMap<String, Vec<String>>),
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::SetPreference(_) => "config.set_preference",
            Command::CheckEnvironment => "config.check_environment",
            Command::Login { .. } => "config.login",
            Command::SendLogin { .. } => "config.send_login",
            Command::PasteLogin { .. } => "config.paste_login",
            Command::ResizeLogin { .. } => "config.resize_login",
            Command::EndLogin => "config.end_login",
            Command::ConnectNotion { .. } | Command::ConnectGithub { .. } | Command::TurnOff(_) => {
                "config.set_task_source"
            }
            Command::ReadSchema(_) => "config.read_schema",
            Command::Map { .. } => "config.map",
            Command::Rebind(_) => "config.rebind",
        }
    }
}

pub fn dispatch(
    command: Command,
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
) {
    match command {
        Command::SetPreference(one) => set_preference(state, one),
        Command::CheckEnvironment => environment::check(state, spawner),
        Command::Login { cols, rows } => environment::login(state, spawner, (cols, rows)),
        Command::SendLogin { bytes } => environment::send(state, &bytes),
        Command::PasteLogin { text } => environment::paste(state, &text),
        Command::ResizeLogin { cols, rows } => environment::resize(state, (cols, rows)),
        Command::EndLogin => environment::end(state),
        Command::ConnectNotion {
            token,
            database_id,
            user_id,
        } => sources::notion(state, spawner, (token, database_id, user_id)),
        Command::ConnectGithub { host } => sources::github(state, spawner, host),
        Command::TurnOff(id) => sources::off(state, services, spawner, id),
        Command::ReadSchema(source) => mapping::read(state, spawner, source),
        Command::Map { source, change } => mapping::map(state, services, spawner, (source, change)),
        Command::Rebind(keymap) => rebind(state, keymap),
    }
}

/// The next key reads the new chords; the file follows.
fn rebind(state: &mut AppState, keymap: BTreeMap<String, Vec<String>>) {
    let Some(config) = state.config.rebind(keymap).cloned() else {
        let e = groove_types::Error::invalid("there is no config to change before the first run");
        return state.failed(e);
    };
    if let Err(e) = groove_config_service::save(&state.env.config_dir, &config) {
        state.failed(e);
    }
}

/// The change reaches every reader at once; the file follows, written on the spot.
fn set_preference(state: &mut AppState, one: Preference) {
    let Some(config) = state.config.set(one).cloned() else {
        let e = groove_types::Error::invalid("there is no config to change before the first run");
        return state.failed(e);
    };
    if let Preference::Theme(theme) = one {
        let palette = groove_agent_service::palette(theme);
        state.agent.recolor(palette);
        state.shell.recolor(palette);
    }
    if let Err(e) = groove_config_service::save(&state.env.config_dir, &config) {
        state.failed(e);
    }
}
