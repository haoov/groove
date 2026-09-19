//! The window's boundaries on disk. The app writes this one; the user writes `config.json`.

use std::path::{Path, PathBuf};

use groove_types::Panes;

use crate::error::{Error, Result};

const FILE_NAME: &str = "panes.json";

/// `<data dir>/panes.json`, the data dir being `~/.local/share/groove`.
pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

/// What the last run left, or `None` when nothing was ever dragged.
pub fn load(path: &Path) -> Result<Option<Panes>> {
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

pub fn save(path: &Path, panes: &Panes) -> Result<()> {
    let text = serde_json::to_string_pretty(panes).map_err(|source| Error::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    write(path, &text).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })
}

fn write(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, text)
}
