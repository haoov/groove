//! The plugins on disk, read and written on the calling thread.

use std::path::{Path, PathBuf};

use groove_types::{Error, ErrorKind, Result, Skill};

use crate::{BUILT_IN, CORE, Dirs, NAMED, USER, is_name, parse, shared};

pub(crate) fn sync(dirs: &Dirs) -> Result<()> {
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
        kept(&skill.join(FILE), body)?;
    }
    Ok(())
}

/// `body` written at `path` unless it holds it already.
fn kept(path: &Path, body: &str) -> std::io::Result<()> {
    match std::fs::read_to_string(path) {
        Ok(held) if held == body => Ok(()),
        _ => std::fs::write(path, body),
    }
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

pub(crate) fn plugin_dirs(dirs: &Dirs) -> Vec<PathBuf> {
    let shared = dirs.shared.iter().map(|one| dirs.built.join(&one.name));
    std::iter::once(dirs.core.clone())
        .chain(shared)
        .chain(std::iter::once(dirs.mine.clone()))
        .filter(|dir| !names(dir).is_empty())
        .collect()
}

pub(crate) fn list(dirs: &Dirs) -> Vec<Skill> {
    let mut out = read_plugin(&dirs.core, CORE, false);
    out.extend(shared::listed(dirs));
    out.extend(
        read_plugin(&dirs.user, USER, true)
            .into_iter()
            .map(|mut one| {
                one.enabled = !dirs.off.contains(&one.id);
                one
            }),
    );
    out
}

pub(crate) fn read(dirs: &Dirs, id: &str) -> Result<String> {
    let path = path_of(dirs, id)?;
    std::fs::read_to_string(&path).map_err(|e| {
        Error::new(
            ErrorKind::Io,
            format!("cannot read {}: {e}", path.display()),
        )
    })
}

pub(crate) fn save(dirs: &Dirs, name: &str, body: &str, instead_of: Option<&str>) -> Result<Skill> {
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
    shared::build(dirs).map_err(io)?;
    Ok(parse::skill(USER, name, body, true, changed_at(&dir)))
}

pub(crate) fn delete(dirs: &Dirs, name: &str) -> Result<()> {
    if !is_name(name) {
        return Err(Error::invalid(format!("{NAMED}: `{name}`")));
    }
    let dir = dirs.user.join("skills").join(name);
    std::fs::remove_dir_all(&dir).map_err(io)?;
    shared::build(dirs).map_err(io)
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
