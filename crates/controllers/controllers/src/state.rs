//! `AppState`: the sum of the services' slices, and the machine it runs on.

use std::path::PathBuf;

use groove_agent_service::{Receiver, Server};

/// The app's directories, and the loopbacks the binary opened for the agents.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Env {
    pub home: PathBuf,
    /// `~/.config/groove`
    pub config_dir: PathBuf,
    /// `~/.local/share/groove`
    pub data_dir: PathBuf,
    pub hooks: Option<Receiver>,
    pub tools: Option<Server>,
    /// The login shell a terminal of the manual section runs.
    pub shell: String,
    /// The kubeconfig files kubectl reads.
    pub kubeconfig: Vec<PathBuf>,
    /// `~/.cache/groove`
    pub cache_dir: PathBuf,
}

impl Env {
    pub fn config_file(&self) -> PathBuf {
        groove_config_service::path(&self.config_dir)
    }

    /// The one database every store shares.
    pub fn database(&self) -> PathBuf {
        self.data_dir.join("app.db")
    }
}

/// The sum of the services' slices, owned on the main thread.
#[derive(Debug, Default)]
pub struct AppState {
    pub env: Env,
    pub focused: bool,
    /// When the user last did anything, which is what the clock counts as work.
    pub acted_at: groove_types::Timestamp,
    /// What a job could not do, newest last; the feed shows them.
    pub errors: Vec<Told<groove_types::Error>>,
    /// What a job wants the user to hear, newest last.
    pub notes: Vec<Told<String>>,
    /// Jobs the user is waiting on, oldest first.
    pub pending: Vec<Pending>,
    next_pending: u64,
    pub task: groove_task_service::State,
    pub session: groove_session_service::State,
    pub workspace: groove_workspace_service::State,
    pub delivery: groove_delivery_service::State,
    pub agent: groove_agent_service::State,
    pub shell: groove_shell_service::State,
    pub config: groove_config_service::State,
    pub cluster: groove_cluster_service::State,
}

/// One job in flight, as the feed names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    pub id: u64,
    pub label: String,
}

/// Something a job left for the user, at the moment it did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Told<T> {
    pub at: groove_types::Timestamp,
    pub what: T,
}

impl<T> Told<T> {
    pub fn now(what: T) -> Self {
        Self {
            at: groove_types::Timestamp::now(),
            what,
        }
    }
}

impl<T> std::ops::Deref for Told<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.what
    }
}

impl AppState {
    pub fn new(env: Env) -> Self {
        Self {
            env,
            ..Self::default()
        }
    }

    /// Marks a job the user waits on; `end` with the id when it lands.
    pub fn begin(&mut self, label: impl Into<String>) -> u64 {
        self.next_pending += 1;
        let id = self.next_pending;
        self.pending.push(Pending {
            id,
            label: label.into(),
        });
        id
    }

    pub fn end(&mut self, id: u64) {
        self.pending.retain(|p| p.id != id);
    }

    /// What a job could not do, for the feed to say.
    pub fn failed(&mut self, error: groove_types::Error) {
        kept(&mut self.errors, Told::now(error));
    }

    /// What a job wants the user to hear.
    pub fn say(&mut self, said: impl Into<String>) {
        kept(&mut self.notes, Told::now(said.into()));
    }
}

/// How many errors and notes the feed keeps.
const TOLD_MAX: usize = 100;

/// One more at the end, the oldest let go past `TOLD_MAX`.
fn kept<T>(list: &mut Vec<Told<T>>, one: Told<T>) {
    list.push(one);
    let over = list.len().saturating_sub(TOLD_MAX);
    list.drain(..over);
}
