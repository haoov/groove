//! The `config` controller: one function per user action on the `config` service.

mod environment;

use groove_config_service::Preference;

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
        }
    }
}

pub fn dispatch(
    command: Command,
    state: &mut AppState,
    _services: &Services,
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
