//! The team's marketplace: the plugins it lists, each one a namespace of skills.

use std::path::{Path, PathBuf};

use groove_types::{Error, Result, Skill};

use crate::{CORE, Dirs, NAMED, USER, is_name, manifest, names, read_plugin};

/// A shared repo read as a marketplace: its own name, and the plugins it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marketplace {
    pub name: String,
    pub plugins: Vec<Plugin>,
}

/// One plugin of the marketplace: its namespace, and the directory it stands in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plugin {
    pub name: String,
    pub dir: PathBuf,
}

/// The marketplace a shared repo is, read from `.claude-plugin/marketplace.json` at its root.
pub fn marketplace(repo: &Path) -> Result<Marketplace> {
    let read = json(&repo.join(".claude-plugin").join("marketplace.json"))
        .map_err(|why| Error::invalid(format!("the shared repo is no marketplace: {why}")))?;
    let name = read["name"].as_str().unwrap_or_default().to_string();
    if !is_name(&name) {
        return Err(Error::invalid(format!("the marketplace's name: {NAMED}")));
    }
    let entries = read["plugins"].as_array().cloned().unwrap_or_default();
    let plugins = entries
        .iter()
        .map(|entry| plugin(repo, entry))
        .collect::<Result<Vec<_>>>()?;
    Ok(Marketplace { name, plugins })
}

/// One entry: a plugin of the repo itself, named the same in its own manifest.
fn plugin(repo: &Path, entry: &serde_json::Value) -> Result<Plugin> {
    let name = entry["name"].as_str().unwrap_or_default();
    if !is_name(name) {
        return Err(Error::invalid(format!("a shared plugin's name: {NAMED}")));
    }
    if name == CORE || name == USER {
        let taken = format!("a shared plugin cannot be named `{name}`: Groove names that one");
        return Err(Error::invalid(taken));
    }
    let source = entry["source"].as_str().unwrap_or_default();
    let inside = source.starts_with("./") && !source.split('/').any(|one| one == "..");
    if !inside {
        let why = format!("`{name}` is not a directory of the repo: Groove reads no other source");
        return Err(Error::invalid(why));
    }
    let dir = repo.join(source.trim_start_matches("./"));
    let own = json(&dir.join(".claude-plugin").join("plugin.json"))
        .map_err(|why| Error::invalid(format!("the plugin `{name}`: {why}")))?;
    if own["name"].as_str() != Some(name) {
        let why = format!("the plugin `{name}` is named otherwise in its own plugin.json");
        return Err(Error::invalid(why));
    }
    Ok(Plugin {
        name: name.to_string(),
        dir,
    })
}

fn json(path: &Path) -> std::result::Result<serde_json::Value, String> {
    let text = std::fs::read_to_string(path).map_err(|_| format!("no {}", file(path)))?;
    serde_json::from_str(&text).map_err(|e| format!("{} does not read: {e}", file(path)))
}

fn file(path: &Path) -> String {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    format!(".claude-plugin/{name}")
}

/// Every skill of every shared plugin, read from the copy; only the ones enabled are on.
pub(crate) fn listed(dirs: &Dirs) -> Vec<Skill> {
    let read = dirs
        .shared
        .iter()
        .flat_map(|plugin| read_plugin(&plugin.dir, &plugin.name, false));
    read.map(|mut one| {
        one.enabled = dirs.enabled.contains(&one.id);
        one
    })
    .collect()
}

/// The plugins a launch is given, built again with the skills switched on alone.
pub(crate) fn build(dirs: &Dirs) -> std::io::Result<()> {
    for out in [&dirs.built, &dirs.mine] {
        if out.exists() {
            std::fs::remove_dir_all(out)?;
        }
    }
    for plugin in &dirs.shared {
        let on = |name: &str| dirs.enabled.contains(&format!("{}:{name}", plugin.name));
        let out = dirs.built.join(&plugin.name);
        linked(
            &plugin.dir,
            &out,
            (&plugin.name, "Shared through Groove"),
            on,
        )?;
    }
    let on = |name: &str| !dirs.off.contains(&format!("{USER}:{name}"));
    linked(
        &dirs.user,
        &dirs.mine,
        (USER, "Your own Groove actions"),
        on,
    )
}

/// A plugin at `out` named `name`, holding links to the skills of `from` that `on` keeps.
fn linked(
    from: &Path,
    out: &Path,
    (name, description): (&str, &str),
    on: impl Fn(&str) -> bool,
) -> std::io::Result<()> {
    let kept: Vec<String> = names(from).into_iter().filter(|one| on(one)).collect();
    if kept.is_empty() {
        return Ok(());
    }
    manifest(out, name, description)?;
    std::fs::create_dir_all(out.join("skills"))?;
    for one in kept {
        let skill = from.join("skills").join(&one);
        std::os::unix::fs::symlink(skill, out.join("skills").join(&one))?;
    }
    Ok(())
}
