//! The files a launch writes beside the agent: the prompt, the plugin dir.

use std::fs::{self, OpenOptions, Permissions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use groove_types::{Error, ErrorKind, Result};

/// The launch files of one session: `<dir>/<session>.<name>`, mode 0600.
pub(crate) struct LaunchDir {
    dir: PathBuf,
    session: String,
}

impl LaunchDir {
    pub fn new(dir: &Path, session: &str) -> Self {
        Self {
            dir: dir.to_path_buf(),
            session: session.to_string(),
        }
    }

    /// Writes the file and returns its path as an argument.
    pub fn write(&self, name: &str, contents: &str) -> Result<String> {
        let path = self.dir.join(format!("{}.{name}", self.session));
        write_private(&path, contents).map_err(|source| {
            Error::new(
                ErrorKind::Agent,
                format!("cannot write {}: {source}", path.display()),
            )
        })?;
        Ok(path.to_string_lossy().into_owned())
    }
}

fn write_private(path: &Path, contents: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.set_permissions(Permissions::from_mode(0o600))?;
    file.write_all(contents.as_bytes())
}
