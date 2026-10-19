//! The `config` controller: one function per user action on the `config` service.

mod clusters;
mod environment;
mod mapping;
mod routines;
pub(crate) mod shared;
mod sources;

use std::collections::BTreeMap;

use groove_config_service::Preference;
use groove_types::{ClusterChange, Error, Mapping, ProviderId, Secret};

use crate::{AppState, Services, Spawner};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `config.set_preference`: one preference changed and the file written.
    SetPreference(Preference),
    /// `config.check_environment`: each program's version and sign-in, read again.
    CheckEnvironment,
    /// `config.login`: `claude auth login` on a terminal of Setup's own.
    Login { cols: u16, rows: u16 },
    /// What goes to the sign-in's terminal while it runs.
    SendLogin { bytes: Vec<u8> },
    /// What is pasted into the sign-in's terminal.
    PasteLogin { text: String },
    /// The sign-in's terminal at a new size.
    ResizeLogin { cols: u16, rows: u16 },
    /// The sign-in ended before it was done.
    EndLogin,
    /// `config.set_task_source`: Notion on, once the token reads the database.
    ConnectNotion {
        token: Secret,
        database_id: String,
        user_id: String,
    },
    /// GitHub on, once the host answers the token `gh` holds for it.
    ConnectGithub { host: String },
    /// A source off, its block gone; the last one stays.
    TurnOff(ProviderId),
    /// `config.read_schema`: the properties a source holds, for the mapping.
    ReadSchema(ProviderId),
    /// `config.map`: one name or value of a source mapped.
    Map { source: ProviderId, change: Mapping },
    /// `config.rebind`: the chords that replace the keymap's defaults, whole.
    Rebind(BTreeMap<String, Vec<String>>),
    /// `config.set_shared`: the team's repo named, once its branch reads as a plugin.
    JoinShared { url: String, branch: String },
    /// `config.set_shared`: no shared repo any more.
    LeaveShared,
    /// `config.switch_routine`: one routine on, its scope approved, or off.
    SwitchRoutine { id: String, on: bool },
    /// `config.switch_trigger`: one trigger of a routine on or off.
    SwitchTrigger {
        id: String,
        trigger: groove_types::Trigger,
        on: bool,
    },
    /// `config.add_cluster`: a kubeconfig context Groove knows from now on, checked at once.
    AddCluster { context: String },
    /// `config.remove_cluster`: a context Groove no longer knows.
    RemoveCluster { context: String },
    /// `config.set_cluster`: one setting of an added context.
    SetCluster {
        context: String,
        change: ClusterChange,
    },
    /// `config.keep_panes`: where the window's boundaries stand, written for the next run.
    KeepPanes(groove_types::Panes),
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
            Command::AddCluster { .. } => "config.add_cluster",
            Command::RemoveCluster { .. } => "config.remove_cluster",
            Command::SetCluster { .. } => "config.set_cluster",
            Command::KeepPanes(_) => "config.keep_panes",
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
        Command::SetPreference(one) => set_preference(state, services, spawner, one),
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
        Command::Rebind(keymap) => rebind(state, services, spawner, keymap),
        Command::JoinShared { url, branch } => shared::join(state, spawner, (url, branch)),
        Command::LeaveShared => shared::leave(state, services, spawner),
        Command::SwitchRoutine { id, on } => routines::switch(state, services, spawner, (&id, on)),
        Command::SwitchTrigger { id, trigger, on } => {
            routines::trigger(state, services, spawner, (&id, trigger, on))
        }
        Command::AddCluster { context } => clusters::add(state, services, spawner, &context),
        Command::RemoveCluster { context } => clusters::remove(state, services, spawner, &context),
        Command::SetCluster { context, change } => {
            clusters::change(state, services, spawner, (&context, change))
        }
        Command::KeepPanes(panes) => keep_panes(state, services, spawner, panes),
    }
}

/// The boundaries held, then written in a job; a failure is said.
fn keep_panes(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    panes: groove_types::Panes,
) {
    services.config.hold_panes(&state.env.data_dir, panes);
    flushed(services, spawner, 0, None);
}

/// What waits for the config file, run once it is on disk.
pub(crate) type Then = Box<dyn FnOnce(&mut AppState, &Services, &dyn Spawner) + Send>;

/// What waits for a config revision to reach the disk, by that revision.
#[derive(Default)]
pub(crate) struct Waiting(Vec<(u64, Then)>);

impl std::fmt::Debug for Waiting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let revisions: Vec<u64> = self.0.iter().map(|(at, _)| *at).collect();
        f.debug_tuple("Waiting").field(&revisions).finish()
    }
}

impl Waiting {
    /// What waits for `kept` or an older revision, taken out in the order it came.
    fn due(&mut self, kept: u64) -> Vec<Then> {
        let (due, left) = std::mem::take(&mut self.0)
            .into_iter()
            .partition(|(at, _)| *at <= kept);
        self.0 = left;
        due.into_iter().map(|(_, then)| then).collect()
    }
}

/// The config as changed, written in a job; false, and said, when there is none before the first run.
pub(crate) fn written(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    changed: Option<groove_types::Config>,
) -> bool {
    let Some(config) = changed else {
        state.failed(Error::invalid(
            "there is no config to change before the first run",
        ));
        return false;
    };
    write(state, services, spawner, config, None);
    true
}

/// `config` written in a job; `then` runs once it, or a later config, is on disk, by any write.
pub(crate) fn write(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    config: groove_types::Config,
    then: Option<Then>,
) {
    let revision = state.config.revised();
    services
        .config
        .hold(&state.env.config_dir, config, revision);
    flushed(services, spawner, revision, then);
}

/// What the keeper holds, written in a job; a failure is said.
fn flushed(services: &Services, spawner: &dyn Spawner, revision: u64, then: Option<Then>) {
    let keeper = services.config.clone();
    spawner.spawn(Box::pin(async move {
        let flushed = keeper.flush().await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                if let Err(e) = flushed {
                    state.failed(e);
                }
                if let Some(then) = then {
                    state.waiting.0.push((revision, then));
                }
                for then in state.waiting.due(services.config.kept()) {
                    then(state, services, spawner);
                }
            },
        ) as crate::Continuation
    }));
}

/// The next key reads the new chords; the file follows.
fn rebind(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    keymap: BTreeMap<String, Vec<String>>,
) {
    let changed = state.config.rebind(keymap).cloned();
    written(state, services, spawner, changed);
}

/// The change reaches every reader at once; the file follows.
fn set_preference(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    one: Preference,
) {
    let changed = state.config.set(one.clone()).cloned();
    if !written(state, services, spawner, changed) {
        return;
    }
    if let Preference::Server(..) = one {
        crate::workspace::servers_reconfigured(state, spawner);
    }
    if let Preference::Theme(theme) = one {
        let palette = groove_agent_service::palette(theme);
        state.agent.recolor(palette);
        state.shell.recolor(palette);
        if let Some(login) = &state.config.login {
            login.recolor(palette);
        }
    }
}
