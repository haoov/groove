//! The `config` controller: one function per user action on the `config` service.

mod environment;
mod mapping;
mod routines;
pub(crate) mod shared;
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
    /// `config.set_shared`: the team's repo named, once its branch reads as a plugin.
    JoinShared {
        url: String,
        branch: String,
    },
    /// `config.set_shared`: no shared repo any more.
    LeaveShared,
    /// `config.switch_routine`: one routine on, its scope approved, or off.
    SwitchRoutine {
        id: String,
        on: bool,
    },
    /// `config.switch_trigger`: one trigger of a routine on or off.
    SwitchTrigger {
        id: String,
        trigger: groove_types::Trigger,
        on: bool,
    },
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
            Command::JoinShared { .. } | Command::LeaveShared => "config.set_shared",
            Command::SwitchRoutine { .. } => "config.switch_routine",
            Command::SwitchTrigger { .. } => "config.switch_trigger",
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
        Command::JoinShared { url, branch } => shared::join(state, spawner, (url, branch)),
        Command::LeaveShared => shared::leave(state, spawner),
        Command::SwitchRoutine { id, on } => routines::switch(state, services, spawner, (&id, on)),
        Command::SwitchTrigger { id, trigger, on } => routines::trigger(state, &id, trigger, on),
    }
}

/// The config as changed, written to its file; whether it was.
pub(crate) fn written(state: &mut AppState, changed: Option<groove_types::Config>) -> bool {
    let kept = groove_config_service::keep(&state.env.config_dir, changed.as_ref());
    kept.map_err(|e| state.failed(e)).is_ok()
}

/// The next key reads the new chords; the file follows.
fn rebind(state: &mut AppState, keymap: BTreeMap<String, Vec<String>>) {
    let changed = state.config.rebind(keymap).cloned();
    written(state, changed);
}

/// The change reaches every reader at once; the file follows, written on the spot.
fn set_preference(state: &mut AppState, one: Preference) {
    let changed = state.config.set(one).cloned();
    if !written(state, changed) {
        return;
    }
    if let Preference::Theme(theme) = one {
        let palette = groove_agent_service::palette(theme);
        state.agent.recolor(palette);
        state.shell.recolor(palette);
    }
}
