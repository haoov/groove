//! The config file. `Config` in `types` is its shape; this is its place on disk.

mod error;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

pub use error::{Error, Result};
use groove_types::Config;

const FILE_NAME: &str = "config.json";

/// `<config dir>/config.json`, the config dir being `~/.config/groove`.
pub fn path(config_dir: &Path) -> PathBuf {
    config_dir.join(FILE_NAME)
}

/// The file's content, or `None` before first run.
pub fn load(path: &Path) -> Result<Option<Config>> {
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
