//! The skills an agent can be sent: the core plugin written at start, and the user's own.

mod parse;
mod shared;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use groove_types::{Error, ErrorKind, Result, Skill};

/// The plugin names, which are also the agent's own namespaces.
pub const CORE: &str = "groove";
pub const USER: &str = "user";

pub use shared::{Marketplace, Plugin, marketplace};

/// The core skills, compiled in and written out over whatever was there.
const BUILT_IN: &[(&str, &str)] = &[
    ("close-task", include_str!("core/close-task.md")),
    ("co-review", include_str!("core/co-review.md")),
    ("convert-explorer", include_str!("core/convert-explorer.md")),
    ("create-task", include_str!("core/create-task.md")),
    ("fix-ci", include_str!("core/fix-ci.md")),
    ("fix-notes", include_str!("core/fix-notes.md")),
    ("new-skill", include_str!("core/new-skill.md")),
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
}

impl Dirs {
    /// `<data>/plugins/groove`, `<config>/user-skills`, and `<data>/plugins/shared`.
    pub fn new(data: &Path, config: &Path) -> Self {
        Self {
            core: data.join("plugins").join(CORE),
            user: config.join("user-skills"),
            shared: Vec::new(),
            built: data.join("plugins").join("shared"),
            enabled: Vec::new(),
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

/// Every plugin on disk: the core one written again, the user's made, the shared ones built.
pub fn sync(dirs: &Dirs) -> Result<()> {
    wrote_core(&dirs.core).map_err(io)?;
    made_user(&dirs.user).map_err(io)?;
    shared::build(dirs).map_err(io)?;
    Ok(())
}

fn wrote_core(dir: &Path) -> std::io::Result<()> {
    manifest(dir, CORE, "Groove workbench actions")?;
    let skills = dir.join("skills");
    std::fs::create_dir_all(&skills)?;
    for name in names(dir) {
        if !BUILT_IN.iter().any(|(one, _)| *one == name) {
            std::fs::remove_dir_all(skills.join(&name))?;
        }
    }
    for (name, body) in BUILT_IN {
        let skill = skills.join(name);
        std::fs::create_dir_all(&skill)?;
        std::fs::write(skill.join(FILE), body)?;
    }
    Ok(())
}

fn made_user(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir.join("skills"))?;
    manifest(dir, USER, "Your own Groove actions")
}

pub(crate) fn manifest(dir: &Path, name: &str, description: &str) -> std::io::Result<()> {
    let meta = dir.join(".claude-plugin");
    std::fs::create_dir_all(&meta)?;
    let body = serde_json::json!({
        "name": name,
        "version": env!("CARGO_PKG_VERSION"),
        "description": description,
        "author": { "name": "Groove" },
    });
    std::fs::write(meta.join("plugin.json"), body.to_string())
}

const FILE: &str = "SKILL.md";

/// The `--plugin-dir` arguments for one launch. A plugin with no skill is left out.
pub fn plugin_dirs(dirs: &Dirs) -> Vec<PathBuf> {
    let shared = dirs.shared.iter().map(|one| dirs.built.join(&one.name));
    std::iter::once(dirs.core.clone())
        .chain(shared)
        .chain(std::iter::once(dirs.user.clone()))
        .filter(|dir| !names(dir).is_empty())
        .collect()
}

/// Every skill the plugins offer: the core ones, the shared ones, then the user's own.
pub fn list(dirs: &Dirs) -> Vec<Skill> {
    let mut out = read_plugin(&dirs.core, CORE, false);
    out.extend(shared::listed(dirs));
    out.extend(read_plugin(&dirs.user, USER, true));
    out
}

/// One skill's own `SKILL.md`, whichever plugin holds it.
pub fn read(dirs: &Dirs, id: &str) -> Result<String> {
    let path = path_of(dirs, id)?;
    std::fs::read_to_string(&path).map_err(|e| {
        Error::new(
            ErrorKind::Io,
            format!("cannot read {}: {e}", path.display()),
        )
    })
}

/// One skill of the user's own written, taking the place of `instead_of` when it names one.
pub fn save(dirs: &Dirs, name: &str, body: &str, instead_of: Option<&str>) -> Result<Skill> {
    if !is_name(name) {
        return Err(Error::invalid(format!("{NAMED}: `{name}`")));
    }
    if body.trim().is_empty() {
        return Err(Error::invalid("a skill with no body is not a skill"));
    }
    let root = dirs.user.join("skills");
    let taken = root.join(name).join(FILE).exists();
    if taken && instead_of != Some(name) {
        return Err(Error::invalid(format!(
            "a skill of yours is already named `{name}`: name it as the one to replace, \
             or choose another"
        )));
    }
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).map_err(io)?;
    std::fs::write(dir.join(FILE), body).map_err(io)?;
    if let Some(gone) = instead_of.filter(|one| *one != name && is_name(one)) {
        std::fs::remove_dir_all(root.join(gone)).map_err(io)?;
    }
    Ok(parse::skill(USER, name, body, true, changed_at(&dir)))
}

/// One skill of the user's own deleted.
pub fn delete(dirs: &Dirs, name: &str) -> Result<()> {
    if !is_name(name) {
        return Err(Error::invalid(format!("{NAMED}: `{name}`")));
    }
    let dir = dirs.user.join("skills").join(name);
    std::fs::remove_dir_all(&dir).map_err(io)
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

/// The file behind a `plugin:name` id.
fn path_of(dirs: &Dirs, id: &str) -> Result<PathBuf> {
    let (plugin, name) = id
        .split_once(':')
        .ok_or_else(|| Error::invalid(format!("not a skill id: `{id}`")))?;
    if !is_name(name) {
        return Err(Error::invalid(format!("{NAMED}: `{name}`")));
    }
    let dir = dirs
        .of(plugin)
        .ok_or_else(|| Error::invalid(format!("no plugin named `{plugin}`")))?;
    Ok(dir.join("skills").join(name).join(FILE))
}

/// The skill directories of one plugin, sorted by name.
pub(crate) fn names(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir.join("skills"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|one| one.path().is_dir())
        .map(|one| one.file_name().to_string_lossy().into_owned())
        .collect();
    out.sort();
    out
}

pub(crate) fn read_plugin(dir: &Path, plugin: &str, editable: bool) -> Vec<Skill> {
    names(dir)
        .into_iter()
        .filter_map(|name| {
            let at = dir.join("skills").join(&name);
            let body = std::fs::read_to_string(at.join(FILE)).ok()?;
            Some(parse::skill(
                plugin,
                &name,
                &body,
                editable,
                changed_at(&at),
            ))
        })
        .collect()
}

/// When the skill's own file was last written.
fn changed_at(dir: &Path) -> groove_types::Timestamp {
    let seconds = std::fs::metadata(dir.join(FILE))
        .and_then(|one| one.modified())
        .ok()
        .and_then(|one| one.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|one| one.as_secs() as i64)
        .unwrap_or_default();
    groove_types::Timestamp::new(seconds)
}

fn io(e: std::io::Error) -> Error {
    Error::new(ErrorKind::Io, e.to_string())
}
