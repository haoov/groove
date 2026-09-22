//! The buttons a note's own row carries.

/// What a note's row of buttons offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteButton {
    Edit,
    Resolve,
    Delete,
    Post,
    Reply,
}

impl NoteButton {
    /// What the button says; a resolved note offers to open again.
    pub fn label(self, resolved: bool) -> &'static str {
        match (self, resolved) {
            (NoteButton::Edit, _) => "edit",
            (NoteButton::Resolve, false) => "resolve",
            (NoteButton::Resolve, true) => "reopen",
            (NoteButton::Delete, _) => "delete",
            (NoteButton::Post, _) => "post",
            (NoteButton::Reply, _) => "reply",
        }
    }
}
