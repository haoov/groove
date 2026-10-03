//! The window's boundaries on disk. The app writes this one; the user writes `config.json`.

use std::path::{Path, PathBuf};

use groove_types::Panes;

use crate::error::Result;

const FILE_NAME: &str = "panes.json";

/// `<data dir>/panes.json`, the data dir being `~/.local/share/groove`.
pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

/// What the last run left, or `None` when nothing was ever dragged.
pub fn load(path: &Path) -> Result<Option<Panes>> {
    crate::load_json(path)
}

pub fn save(path: &Path, panes: &Panes) -> Result<()> {
    crate::save_json(path, panes)
}
