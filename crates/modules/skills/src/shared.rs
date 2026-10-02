//! The team's marketplace: the plugins it lists, each one a namespace of skills.

use std::path::{Path, PathBuf};

use groove_types::{Error, Result};

use crate::{CORE, NAMED, USER, is_name};

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
