//! The buttons a thread's own row carries.

/// What a thread's row of buttons offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteButton {
    Resolve,
    Reply,
    /// A resolved thread folded to one line, or opened out of it.
    Fold,
}

impl NoteButton {
    /// What the button says; a resolved thread offers to open again.
    pub fn label(self, resolved: bool) -> &'static str {
        match (self, resolved) {
            (NoteButton::Resolve, false) => "resolve",
            (NoteButton::Resolve, true) => "reopen",
            (NoteButton::Reply, _) => "reply",
            (NoteButton::Fold, _) => "fold",
        }
    }
}

/// What the composer of a reply offers: send and cancel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Composing {
    Save,
    Cancel,
}
