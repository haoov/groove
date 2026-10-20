//! The config file. `Config` in `types` is its shape; this is its place on disk.

mod check;
mod error;
pub mod panes;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

pub use check::{check, versions};
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

/// The whole file written again off the calling thread; the old one stays until the new is on disk.
pub async fn write(path: &Path, config: &Config) -> Result<()> {
    write_json(path, config).await
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

/// A JSON file written whole, pretty, its directory made when missing; the old one stays until the new is on disk.
pub(crate) async fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let text = serde_json::to_string_pretty(value).map_err(|source| Error::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    let failed = |source| Error::Write {
        path: path.to_path_buf(),
        source,
    };
    if let Some(dir) = path.parent() {
        tokio::fs::create_dir_all(dir).await.map_err(failed)?;
    }
    let mut beside = path.as_os_str().to_owned();
    beside.push(".tmp");
    let beside = PathBuf::from(beside);
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(&beside)
        .await
        .map_err(failed)?;
    let private = std::os::unix::fs::PermissionsExt::from_mode(0o600);
    file.set_permissions(private).await.map_err(failed)?;
    tokio::io::AsyncWriteExt::write_all(&mut file, text.as_bytes())
        .await
        .map_err(failed)?;
    file.sync_all().await.map_err(failed)?;
    tokio::fs::rename(&beside, path).await.map_err(failed)
}
