//! What frames a note's words: the header a comment opens on, the room around the card.

#[derive(Debug, Clone, Default)]
pub struct Header {
    pub initials: String,
    pub author: String,
    /// `local` for the session's own note, `MR` for a thread.
    pub source: &'static str,
    /// Its Conventional Comments label, and the decoration after it.
    pub label: Option<(String, Option<String>)>,
    /// The lines the note is about, on its opening comment.
    pub lines: String,
    pub at: groove_types::Timestamp,
    /// A reply's header, ruled off from what stands above it.
    pub reply: bool,
    pub replies: usize,
    /// The note a click on a resolved one's header or fold acts on.
    pub folds: Option<groove_types::NoteOrigin>,
}

/// A row of room a note keeps: above its card, below it, or ruled across it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Room {
    Above,
    Between,
    Below,
}
