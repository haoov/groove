//! What frames a note's words: the header a comment opens on, the room around the card.

#[derive(Debug, Clone, Default)]
pub struct Header {
    pub initials: String,
    pub author: String,
    /// `local` for the session's own note, `MR` for a thread.
    pub source: &'static str,
    /// Its Conventional Comments label, and the decorations after it.
    pub label: Option<(String, Vec<String>)>,
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

/// A row of the composer around the words being typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Composed {
    /// The lines it is about, and the label its words open on.
    Chips {
        lines: String,
        label: Option<String>,
    },
    /// The decorations once a label is chosen, with those held; a hint; save and cancel.
    Footer {
        hint: &'static str,
        save: &'static str,
        decorations: Option<Vec<String>>,
    },
}

/// A row of the field the words are typed in: whether it opens it, closes it, and has the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Field {
    pub first: bool,
    pub last: bool,
    pub lit: bool,
}

/// What a note row carries: who said it, whether it opens the note, and its state.
#[derive(Debug, Clone, Copy)]
pub struct Noted<'a> {
    pub author: &'a str,
    /// The lines the note is about, on the row that opens it.
    pub lines: &'a str,
    pub opens: bool,
    pub resolved: bool,
    /// The words as Markdown; the plain text is drawn when there is none.
    pub prose: Option<&'a groove_ui_kit::markdown::Row>,
    pub head: Option<&'a Header>,
    /// A resolved note shown as one line.
    pub folded: bool,
    /// A row of the field the words are typed in.
    pub field: Option<Field>,
}
