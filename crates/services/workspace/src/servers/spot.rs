//! The one spot of a file the popup speaks of, and where the last definition landed.

use groove_types::Caret;

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
