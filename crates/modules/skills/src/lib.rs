//! The skills an agent can be sent: the core plugin written at start, and the user's own.

mod disk;
mod parse;
mod shared;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use groove_types::{Error, ErrorKind, Result, Skill};

/// The plugin names, which are also the agent's own namespaces.
pub const CORE: &str = "groove";
pub const USER: &str = "user";

pub use shared::{Marketplace, Plugin};

/// The core skills, compiled in and written out over whatever was there.
const BUILT_IN: &[(&str, &str)] = &[
    ("close-task", include_str!("core/close-task.md")),
    ("co-review", include_str!("core/co-review.md")),
    ("convert-explorer", include_str!("core/convert-explorer.md")),
    ("create-task", include_str!("core/create-task.md")),
    ("fix-ci", include_str!("core/fix-ci.md")),
    ("fix-notes", include_str!("core/fix-notes.md")),
    ("investigate", include_str!("core/investigate.md")),
    ("new-skill", include_str!("core/new-skill.md")),
    ("promote-fact", include_str!("core/promote-fact.md")),
    ("save-task", include_str!("core/save-task.md")),
    ("start-task", include_str!("core/start-task.md")),
];

/// Where the plugins live: the core one, the user's, and the shared ones with their built copies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    pub core: PathBuf,
    pub user: PathBuf,
    /// The shared repo's plugins, as its copy holds them.
    pub shared: Vec<Plugin>,
    /// Where each shared plugin is built with its enabled skills alone.
    pub built: PathBuf,
    /// The shared skills given to sessions, by id.
    pub enabled: Vec<String>,
    /// Where the user's plugin is built with the skills they did not switch off.
    pub mine: PathBuf,
    /// The user's own skills switched off, by id.
    pub off: Vec<String>,
}

impl Dirs {
    /// `<data>/plugins/groove`, `<config>/user-skills`, and the built ones under `<data>/plugins`.
    pub fn new(data: &Path, config: &Path) -> Self {
        Self {
            core: data.join("plugins").join(CORE),
            user: config.join("user-skills"),
            shared: Vec::new(),
            built: data.join("plugins").join("shared"),
            enabled: Vec::new(),
            mine: data.join("plugins").join(USER),
            off: Vec::new(),
        }
    }

    /// The same, with the shared repo's plugins and the skills of them enabled.
    pub fn sharing(self, shared: Vec<Plugin>, enabled: Vec<String>) -> Self {
        Self {
            shared,
            enabled,
            ..self
        }
    }

    /// The same, with these skills of the user's own switched off.
    pub fn switched(self, off: Vec<String>) -> Self {
        Self { off, ..self }
    }

    fn of(&self, plugin: &str) -> Option<&Path> {
        match plugin {
            CORE => Some(&self.core),
            USER => Some(&self.user),
            _ => self
                .shared
                .iter()
                .find(|one| one.name == plugin)
                .map(|one| one.dir.as_path()),
        }
    }
}

/// Held while a build runs: one at a time.
static BUILDING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Every plugin on disk: the core one written again, the user's made, the shared ones built.
pub async fn sync(dirs: &Dirs) -> Result<()> {
    let dirs = dirs.clone();
    built(move || disk::sync(&dirs)).await
}

/// The `--plugin-dir` arguments for one launch. A plugin with no skill is left out.
pub async fn plugin_dirs(dirs: &Dirs) -> Result<Vec<PathBuf>> {
    let dirs = dirs.clone();
    off(move || Ok(disk::plugin_dirs(&dirs))).await
}

/// Every skill the plugins offer: the core ones, the shared ones, then the user's own.
pub async fn list(dirs: &Dirs) -> Result<Vec<Skill>> {
    let dirs = dirs.clone();
    off(move || Ok(disk::list(&dirs))).await
}

/// One skill's own `SKILL.md`, whichever plugin holds it.
pub async fn read(dirs: &Dirs, id: &str) -> Result<String> {
    let (dirs, id) = (dirs.clone(), id.to_string());
    off(move || disk::read(&dirs, &id)).await
}

/// One skill of the user's own written, taking the place of `instead_of` when it names one.
pub async fn save(dirs: &Dirs, name: &str, body: &str, instead_of: Option<&str>) -> Result<Skill> {
    let (dirs, name, body) = (dirs.clone(), name.to_string(), body.to_string());
    let instead_of = instead_of.map(str::to_string);
    built(move || disk::save(&dirs, &name, &body, instead_of.as_deref())).await
}

/// One skill of the user's own deleted.
pub async fn delete(dirs: &Dirs, name: &str) -> Result<()> {
    let (dirs, name) = (dirs.clone(), name.to_string());
    built(move || disk::delete(&dirs, &name)).await
}

/// The marketplace a shared repo is, read from `.claude-plugin/marketplace.json` at its root.
pub async fn marketplace(repo: &Path) -> Result<Marketplace> {
    let repo = repo.to_path_buf();
    off(move || shared::marketplace(&repo)).await
}

async fn built<T: Send + 'static>(work: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    let _one = BUILDING.lock().await;
    off(work).await
}

async fn off<T: Send + 'static>(work: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    match tokio::task::spawn_blocking(work).await {
        Ok(done) => done,
        Err(e) => Err(Error::new(ErrorKind::Io, e.to_string())),
    }
}

const NAMED: &str = "a name is lower case letters, digits and dashes";

/// Lower case, digits and dashes: the name is a path segment.
pub fn is_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}
