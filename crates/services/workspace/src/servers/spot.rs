//! The one spot of a file the popup speaks of, and where the last definition landed.

use std::path::Path;

use groove_types::{Caret, Error, ErrorKind, Location};

use super::Servers;

/// What the servers said of one spot of a file, asked once the pointer rested on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hovered {
    pub path: String,
    pub at: Caret,
    pub said: Option<String>,
    /// Asked from the keyboard at the caret, not by a resting pointer.
    pub caret: bool,
    /// What Groove itself says of the spot, as a definition it could not open.
    pub told: bool,
}

impl Servers {
    /// A spot asked about, its answer not read yet.
    pub fn asked(&mut self, path: &str, at: Caret, caret: bool) {
        self.hovered = Some(Hovered {
            path: path.to_string(),
            at,
            said: None,
            caret,
            told: false,
        });
    }

    /// Groove's own word about a spot, shown where the pointer stands on it.
    pub fn told(&mut self, path: &str, at: Caret, said: String) {
        self.hovered = Some(Hovered {
            path: path.to_string(),
            at,
            said: Some(said),
            caret: false,
            told: true,
        });
    }

    /// A definition opened at `at` of `path`.
    pub fn landed(&mut self, path: &str, at: Caret) {
        self.landed = Some((path.to_string(), at));
        self.landings += 1;
    }

    /// The answer about a spot, kept while that spot is still the one asked about.
    pub fn answered(&mut self, path: &str, at: Caret, said: Option<String>) {
        let asked = self.hovered.as_mut();
        if let Some(hovered) = asked.filter(|one| one.path == path && one.at == at) {
            hovered.said = said;
        }
    }
}

/// Where a definition asked from a spot leads: a place of the worktree, or what to say there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Landing {
    Open(String, Caret),
    Said(String),
}

/// The first of `found` as a place under `dir`, or why it opens nothing.
pub fn landing(dir: &Path, found: &[Location]) -> Landing {
    let Some(first) = found.first() else {
        return Landing::Said("No definition here".into());
    };
    match first.path.strip_prefix(dir) {
        Ok(inside) => Landing::Open(inside.to_string_lossy().into_owned(), first.start),
        Err(_) => Landing::Said(format!(
            "Defined outside the worktree, in {}. Groove opens worktree paths only.",
            first.path.display()
        )),
    }
}

/// What a definition says when no server of `path` is up.
pub fn none_up(path: &str) -> String {
    format!("No language server is up for {path}")
}

/// Why a server did not start; one not installed says how to get it.
pub(super) fn unstarted(name: &str, e: Error) -> Error {
    let said = match e.kind {
        ErrorKind::NotFound => format!(
            "{name} is not installed: {}. Install it, or set its program under `language_servers` in the config.",
            e.message
        ),
        _ => format!("{name} did not start: {}", e.message),
    };
    Error::new(e.kind, said)
}
