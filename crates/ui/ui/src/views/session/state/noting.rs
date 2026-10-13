//! A note being typed in the surface, and what its words become.

use groove_controllers::workspace_service::Buffer;
use groove_types::Anchor;

/// A note being typed in the surface: the lines it is about, and its words.
#[derive(Debug, Clone)]
pub struct Noting {
    pub anchor: Anchor,
    pub buffer: Buffer,
    pub writing: Writing,
}

impl PartialEq for Noting {
    fn eq(&self, other: &Self) -> bool {
        let held = |one: &Self| (one.buffer.text(), one.buffer.selections().to_vec());
        (&self.anchor, &self.writing) == (&other.anchor, &other.writing)
            && held(self) == held(other)
    }
}

impl Eq for Noting {}

/// What the words being typed become.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Writing {
    #[default]
    New,
    /// The note they replace.
    Over(groove_types::AnnotationId),
    /// The thread they answer.
    Reply(String),
}

impl Noting {
    pub fn new(anchor: Anchor) -> Self {
        Self {
            anchor,
            buffer: Buffer::default(),
            writing: Writing::New,
        }
    }

    /// A note's own words, opened to be written again.
    pub fn over(anchor: Anchor, id: groove_types::AnnotationId, said: &str) -> Self {
        Self {
            anchor,
            buffer: Buffer::plain(said),
            writing: Writing::Over(id),
        }
    }

    /// An empty row under a thread, for the words that answer it.
    pub fn reply(anchor: Anchor, thread: String) -> Self {
        Self {
            anchor,
            buffer: Buffer::default(),
            writing: Writing::Reply(thread),
        }
    }

    /// The note it stands over, when it stands over one.
    pub fn over_id(&self) -> Option<&groove_types::AnnotationId> {
        match &self.writing {
            Writing::Over(id) => Some(id),
            _ => None,
        }
    }

    /// What the note says, with nothing around it.
    pub fn said(&self) -> String {
        self.buffer.text().trim().to_string()
    }
}
