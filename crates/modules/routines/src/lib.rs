//! Routines on disk: the team's in the shared copy, the user's own under the config.

mod parse;
mod run;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use groove_types::Routine;

pub use parse::parse;
pub use run::{new_day, prompt, started_by};

/// Where a routine comes from, which names it: `shared:<name>`, `user:<name>`.
pub const SHARED: &str = "shared";
pub const USER: &str = "user";

/// Where the routines are read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    /// `routines/` in the shared copy, when there is a shared repo.
    pub shared: Option<PathBuf>,
    pub user: PathBuf,
}

impl Dirs {
    /// `<config>/routines`, and `routines/` of the shared copy when there is one.
    pub fn new(config: &Path, copy: Option<&Path>) -> Self {
        Self {
            shared: copy.map(|one| one.join("routines")),
            user: config.join("routines"),
        }
    }
}

/// One routine file: the routine it says, or why it says none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub id: String,
    pub read: Result<Routine, String>,
}

/// Every routine file, the team's first, each sorted by name.
pub async fn list(dirs: &Dirs) -> Vec<Listed> {
    let mut out = Vec::new();
    if let Some(dir) = &dirs.shared {
        out.extend(read_dir(dir, SHARED).await);
    }
    out.extend(read_dir(&dirs.user, USER).await);
    out
}

async fn read_dir(dir: &Path, from: &str) -> Vec<Listed> {
    let mut files = Vec::new();
    if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
        while let Ok(Some(one)) = entries.next_entry().await {
            files.push(one.path());
        }
    }
    files.retain(|path| path.extension().is_some_and(|ext| ext == "md"));
    files.sort();
    let mut out = Vec::new();
    for path in files {
        let Some(name) = path
            .file_stem()
            .map(|one| one.to_string_lossy().into_owned())
        else {
            continue;
        };
        let id = format!("{from}:{name}");
        let read = match tokio::fs::read_to_string(&path).await {
            Ok(text) => parse(&id, &name, &text),
            Err(e) => Err(format!("cannot read it: {e}")),
        };
        out.push(Listed { id, read });
    }
    out
}
