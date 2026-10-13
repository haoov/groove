//! The header a comment of a note opens on.

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
}
