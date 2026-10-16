//! The language servers each worktree runs, and what they last said of each file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use groove_lsp::{Client, Hooks, Launch, start};
use groove_types::{Caret, Diagnostic, LanguageServer, Result, WorktreeId};

/// One server: the worktree it serves, the directory it runs in, and its name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServerKey {
    pub worktree: WorktreeId,
    pub dir: PathBuf,
    pub name: String,
}

/// What the editor did to a file, told to its servers. Paths are the worktree's own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tell {
    Opened {
        path: String,
        language: &'static str,
        text: String,
    },
    Changed {
        path: String,
        text: String,
    },
    Saved {
        path: String,
    },
    Closed {
        path: String,
    },
}

/// What telling a server came to.
#[derive(Debug, PartialEq, Eq)]
pub enum Telling {
    /// It was told, will be once it has started, or had nothing to hear.
    Told,
    /// It is not running: start it, and it is told once it has.
    Start,
    /// It failed or stopped; nothing is told until it is started again.
    Down,
}

enum Running {
    Starting(Vec<Tell>),
    Ready(Arc<Client>),
    Down,
}

impl std::fmt::Debug for Running {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Running::Starting(_) => "Starting",
            Running::Ready(_) => "Ready",
            Running::Down => "Down",
        })
    }
}

/// What the servers said of one spot of a file, asked once the pointer rested on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hovered {
    pub path: String,
    pub at: Caret,
    pub said: Option<String>,
    /// Asked from the keyboard at the caret, not by a resting pointer.
    pub caret: bool,
    /// What Groove itself says of the spot, as a definition it could not open.
    pub told: bool,
}

#[derive(Debug, Default)]
pub struct Servers {
    pub hovered: Option<Hovered>,
    /// Where the last definition landed, and how many have, for the view to centre each once.
    pub landed: Option<(String, Caret)>,
    pub landings: u64,
    running: BTreeMap<ServerKey, Running>,
    /// Each file's diagnostics, by the server that found them.
    found: BTreeMap<(WorktreeId, String), BTreeMap<String, Vec<Diagnostic>>>,
}

impl Servers {
    /// Tells `key` what the editor did, or queues it while the server starts. Only an open
    /// starts one, a server that is down included.
    pub fn tell(&mut self, key: &ServerKey, told: Tell) -> Telling {
        match self.running.get_mut(key) {
            Some(Running::Ready(client)) => {
                let client = client.clone();
                if delivered(&client, &key.dir, &told).is_err() {
                    self.running.insert(key.clone(), Running::Down);
                    return Telling::Down;
                }
                Telling::Told
            }
            Some(Running::Starting(queued)) => {
                queued.push(told);
                Telling::Told
            }
            Some(Running::Down) if !matches!(told, Tell::Opened { .. }) => Telling::Down,
            None if !matches!(told, Tell::Opened { .. }) => Telling::Told,
            Some(Running::Down) | None => {
                self.running
                    .retain(|one, _| !(one.worktree == key.worktree && one.name == key.name));
                self.running
                    .insert(key.clone(), Running::Starting(vec![told]));
                Telling::Start
            }
        }
    }

    /// What a start came to: the server is told what waited, or is down.
    pub fn started(&mut self, key: &ServerKey, client: Result<Client>) -> Result<()> {
        let Some(Running::Starting(queued)) = self.running.remove(key) else {
            return Ok(());
        };
        let client = match client {
            Ok(client) => Arc::new(client),
            Err(e) => {
                self.running.insert(key.clone(), Running::Down);
                return Err(e);
            }
        };
        let told = queued
            .iter()
            .try_for_each(|one| delivered(&client, &key.dir, one));
        let running = match told {
            Ok(()) => Running::Ready(client),
            Err(_) => Running::Down,
        };
        self.running.insert(key.clone(), running);
        Ok(())
    }

    /// The server stopped talking. Its diagnostics go with it.
    pub fn closed(&mut self, key: &ServerKey) {
        if let Some(running) = self.running.get_mut(key) {
            *running = Running::Down;
        }
        for ((worktree, _), by) in self.found.iter_mut() {
            if worktree == &key.worktree {
                by.remove(&key.name);
            }
        }
        self.found.retain(|_, by| !by.is_empty());
    }

    /// The server is asked for again: the next file it serves starts it.
    pub fn restart(&mut self, key: &ServerKey) {
        self.running.remove(key);
    }

    /// The client of a server that is up.
    pub fn client(&self, key: &ServerKey) -> Option<Arc<Client>> {
        match self.running.get(key)? {
            Running::Ready(client) => Some(client.clone()),
            _ => None,
        }
    }

    /// The clients of `worktree` that are up, among the servers `names` names.
    pub fn clients(&self, worktree: &WorktreeId, names: &[String]) -> Vec<Arc<Client>> {
        let mut up = Vec::new();
        for (key, running) in &self.running {
            if let (true, Running::Ready(client)) = (&key.worktree == worktree, running)
                && names.contains(&key.name)
            {
                up.push(client.clone());
            }
        }
        up
    }

    /// A spot asked about, its answer not read yet.
    pub fn asked(&mut self, path: &str, at: Caret, caret: bool) {
        self.hovered = Some(Hovered {
            path: path.to_string(),
            at,
            said: None,
            caret,
            told: false,
        });
    }

    /// Groove's own word about a spot, shown where the pointer stands on it.
    pub fn told(&mut self, path: &str, at: Caret, said: String) {
        self.hovered = Some(Hovered {
            path: path.to_string(),
            at,
            said: Some(said),
            caret: false,
            told: true,
        });
    }

    /// A definition opened at `at` of `path`.
    pub fn landed(&mut self, path: &str, at: Caret) {
        self.landed = Some((path.to_string(), at));
        self.landings += 1;
    }

    /// The answer about a spot, kept while that spot is still the one asked about.
    pub fn answered(&mut self, path: &str, at: Caret, said: Option<String>) {
        let asked = self.hovered.as_mut();
        if let Some(hovered) = asked.filter(|one| one.path == path && one.at == at) {
            hovered.said = said;
        }
    }

    pub fn is_down(&self, key: &ServerKey) -> bool {
        matches!(self.running.get(key), Some(Running::Down))
    }

    /// What `key` said of the file at `path`, which a path outside its directory never is.
    pub fn diagnosed(&mut self, key: &ServerKey, path: &Path, found: Vec<Diagnostic>) {
        let Ok(inside) = path.strip_prefix(&key.dir) else {
            return;
        };
        let file = (key.worktree.clone(), inside.to_string_lossy().into_owned());
        let by = self.found.entry(file).or_default();
        match found.is_empty() {
            true => by.remove(&key.name),
            false => by.insert(key.name.clone(), found),
        };
        self.found.retain(|_, by| !by.is_empty());
    }

    /// Every server's diagnostics of one file, in reading order.
    pub fn diagnostics(&self, worktree: &WorktreeId, path: &str) -> Vec<Diagnostic> {
        let Some(by) = self.found.get(&(worktree.clone(), path.to_string())) else {
            return Vec::new();
        };
        let mut all: Vec<Diagnostic> = by.values().flatten().cloned().collect();
        all.sort_by_key(|one| (one.start, one.severity));
        all
    }

    /// A worktree gone: its servers are killed, as dropping their clients does.
    pub fn forget(&mut self, worktree: &WorktreeId) {
        self.running.retain(|key, _| &key.worktree != worktree);
        self.found.retain(|(one, _), _| one != worktree);
    }
}

/// The servers of `language`, in the order the config names them.
pub fn serving<'a>(servers: &'a [LanguageServer], language: &str) -> Vec<&'a LanguageServer> {
    servers.iter().filter(|one| one.serves(language)).collect()
}

/// How to start `server` in `dir`.
pub fn launch(server: &LanguageServer, dir: &Path) -> Launch {
    Launch {
        program: server.program.clone(),
        args: server.args.clone(),
        unset: server.unset.clone(),
        root: dir.to_path_buf(),
        settings: server.settings.clone(),
    }
}

fn delivered(client: &Client, dir: &Path, told: &Tell) -> Result<()> {
    match told {
        Tell::Opened {
            path,
            language,
            text,
        } => client.open(&dir.join(path), language, text.clone()),
        Tell::Changed { path, text } => client.change(&dir.join(path), text.clone()),
        Tell::Saved { path } => client.save(&dir.join(path)),
        Tell::Closed { path } => client.close(&dir.join(path)),
    }
}
