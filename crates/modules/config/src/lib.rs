//! The config file. `Config` in `types` is its shape; this is its place on disk.

mod check;
mod error;
pub mod panes;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

pub use check::check;
pub use error::{Error, Result};
use groove_types::Config;

const FILE_NAME: &str = "config.json";

/// `<config dir>/config.json`, the config dir being `~/.config/groove`.
pub fn path(config_dir: &Path) -> PathBuf {
    config_dir.join(FILE_NAME)
}

/// The file's content, or `None` before first run.
pub fn load(path: &Path) -> Result<Option<Config>> {
    load_json(path)
}

/// The whole file written again, pretty, its directory made when missing.
pub fn save(path: &Path, config: &Config) -> Result<()> {
    save_json(path, config)
}

/// A JSON file read back, or `None` when there is none yet.
pub(crate) fn load_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(Error::Read {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|source| Error::Parse {
            path: path.to_path_buf(),
            source,
        })
}

/// A JSON file written whole, pretty, its directory made when missing.
pub(crate) fn save_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let text = serde_json::to_string_pretty(value).map_err(|source| Error::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    let written = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(path, text));
    written.map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })
}
