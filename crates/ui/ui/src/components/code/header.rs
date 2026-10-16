//! What frames a note's words: the header a comment opens on, the room around the card.

use groove_controllers::delivery_service::BY_USER;
use groove_types::Note;
use groove_ui_kit::base::style::Role;

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
}

/// A row of room inside a note's card: plain, or ruled across it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Room {
    Gap,
    Between,
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
    pub resolved: bool,
    /// The words as Markdown; the plain text is drawn when there is none.
    pub prose: Option<&'a groove_ui_kit::markdown::Row>,
    pub head: Option<&'a Header>,
    /// A row of the field the words are typed in.
    pub field: Option<Field>,
}

impl Header {
    /// The header of `note`'s comment `at`, standing on `lines`.
    pub fn of(note: &Note, at: usize, lines: String) -> Self {
        let Some(said) = note.said.get(at) else {
            return Self::default();
        };
        let label = groove_types::labelled(&said.body).map(|one| {
            let decorations = one.decorations.iter().map(|one| one.to_string());
            (one.label.to_string(), decorations.collect())
        });
        let author = match said.author.as_str() {
            BY_USER => "me",
            other => other,
        };
        Self {
            initials: initials(author),
            author: author.to_string(),
            source: if note.is_local() { "local" } else { "MR" },
            label,
            lines,
            at: said.at,
            reply: at > 0,
        }
    }
}

/// Two capitals for a name: its first two words' first letters, or a lone word's first two.
fn initials(name: &str) -> String {
    let words: Vec<&str> = name
        .split(|one: char| !one.is_alphanumeric())
        .filter(|one| !one.is_empty())
        .collect();
    let letters: String = match words.as_slice() {
        [one] => one.chars().take(2).collect(),
        many => many
            .iter()
            .take(2)
            .filter_map(|word| word.chars().next())
            .collect(),
    };
    letters.to_uppercase()
}

/// The colour a Conventional Comments label reads in.
pub fn label_role(label: &str) -> Role {
    match label {
        "issue" => Role::Bad,
        "suggestion" => Role::Working,
        "question" => Role::Attention,
        "todo" => Role::Warn,
        "praise" => Role::Ok,
        _ => Role::Faint,
    }
}
