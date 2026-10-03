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
pub fn list(dirs: &Dirs) -> Vec<Listed> {
    let shared = dirs.shared.iter().flat_map(|dir| read_dir(dir, SHARED));
    shared.chain(read_dir(&dirs.user, USER)).collect()
}

fn read_dir(dir: &Path, from: &str) -> Vec<Listed> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|one| one.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    files.sort();
    files
        .into_iter()
        .filter_map(|path| {
            let name = path.file_stem()?.to_string_lossy().into_owned();
            let id = format!("{from}:{name}");
            let read = match std::fs::read_to_string(&path) {
                Ok(text) => parse(&id, &name, &text),
                Err(e) => Err(format!("cannot read it: {e}")),
            };
            Some(Listed { id, read })
        })
        .collect()
}
