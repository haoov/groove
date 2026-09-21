use crate::{AnnotationId, Timestamp};

/// Where a note sits in a file, on the new side.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Anchor {
    pub path: String,
    pub start_line: u32,
    pub end_line: u32,
}

impl Anchor {
    /// One line of a file.
    pub fn line(path: impl Into<String>, line: u32) -> Self {
        Anchor {
            path: path.into(),
            start_line: line,
            end_line: line,
        }
    }

    pub fn rows(&self) -> std::ops::RangeInclusive<u32> {
        self.start_line..=self.end_line
    }
}

/// Whose note it is: this session's own, or a thread on the merge request.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum NoteOrigin {
    Local(AnnotationId),
    Thread(String),
}

/// One thing said in a note: an annotation says one, a thread says several.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Said {
    pub author: String,
    pub body: String,
    pub at: Timestamp,
}

/// A note to act on, wherever it came from. `anchor` is absent on a comment left
/// on the merge request itself.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Note {
    pub origin: NoteOrigin,
    pub anchor: Option<Anchor>,
    pub resolved: bool,
    pub said: Vec<Said>,
}

impl Note {
    pub fn is_local(&self) -> bool {
        matches!(self.origin, NoteOrigin::Local(_))
    }

    /// The note's own id, for the writes that take one.
    pub fn id(&self) -> Option<&AnnotationId> {
        match &self.origin {
            NoteOrigin::Local(id) => Some(id),
            NoteOrigin::Thread(_) => None,
        }
    }

    /// What the list shows first: the words it opens with.
    pub fn opening(&self) -> Option<&Said> {
        self.said.first()
    }

    pub fn at(&self) -> Option<Timestamp> {
        self.opening().map(|said| said.at)
    }

    /// How many replies stand under the opening words.
    pub fn replies(&self) -> usize {
        self.said.len().saturating_sub(1)
    }

    /// Whether the note stands on any of these rows of this file.
    pub fn over(&self, path: &str, lines: (u32, u32)) -> bool {
        self.anchor.as_ref().is_some_and(|one| {
            one.path == path && one.start_line <= lines.1 && lines.0 <= one.end_line
        })
    }

    /// Whether the note stands on this row of this file.
    pub fn on(&self, path: &str, line: u32) -> bool {
        self.anchor
            .as_ref()
            .is_some_and(|one| one.path == path && one.rows().contains(&line))
    }
}
