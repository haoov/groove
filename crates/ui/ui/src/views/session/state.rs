//! What the session surface remembers between frames.

use groove_types::{Anchor, DiffView};

pub use super::tab::{Face, Tab};

use super::find::Finding;
use groove_ui_kit::widgets::Field;

/// What the session surface remembers between frames.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SessionUi {
    pub tab: Tab,
    /// How many characters a note row holds, as the last frame measured it.
    pub note_cols: usize,
    /// How far the sidebar's list is scrolled, in pixels.
    pub files: f32,
    /// The user folded the sidebar away.
    pub folded: bool,
    /// The manual section stands open under the tab.
    pub manual: bool,
    /// How far the stream is scrolled, in pixels.
    pub diff: f32,
    /// How far the active file is scrolled, in pixels.
    pub file: f32,
    /// How far the stream's text and the file's are scrolled sideways, in pixels.
    pub diff_across: f32,
    pub file_across: f32,
    /// The caret the file view last scrolled to.
    pub followed: Option<groove_types::Caret>,
    /// The line the caret rests on, and the frame tick it came to rest at.
    pub rest: Option<(crate::views::session::diff::Spot, u64)>,
    /// How far the overview is scrolled, in pixels.
    pub overview: f32,
    /// Which of the two views the stream is drawn in.
    pub view: DiffView,
    /// The keyboard is in the commit box.
    pub composing: bool,
    /// What the sidebar's search bar narrows by.
    pub bar: Bar,
    /// The files whose found lines are hidden under their own row.
    pub shut: std::collections::BTreeSet<String>,
    /// The bar over the rows, while a search of them is live.
    pub find: Option<Finding>,
    /// Which of the sidebar's lists is up.
    pub pane: Pane,
    /// The explorer's own directories that stand open.
    pub opened: std::collections::BTreeSet<String>,
    /// The changed list's directories folded over their files.
    pub closed: std::collections::BTreeSet<String>,
    /// A path being named, where the tree asked for it.
    pub naming: Option<Naming>,
    /// A note being typed, on the lines it will stand on.
    pub noting: Option<Noting>,
}

/// A note being typed in the surface: the lines it is about, and its words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Noting {
    pub anchor: Anchor,
    pub field: Field,
    pub writing: Writing,
}

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
            field: Field::default(),
            writing: Writing::New,
        }
    }

    /// A note's own words, opened to be written again.
    pub fn over(anchor: Anchor, id: groove_types::AnnotationId, said: &str) -> Self {
        let mut field = Field::default();
        field.set(said);
        Self {
            anchor,
            field,
            writing: Writing::Over(id),
        }
    }

    /// An empty row under a thread, for the words that answer it.
    pub fn reply(anchor: Anchor, thread: String) -> Self {
        Self {
            anchor,
            field: Field::default(),
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
    pub fn said(&self) -> &str {
        self.field.text().trim()
    }
}

/// A name being typed in the tree: what it is for, and where it stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Naming {
    pub asked: Asked,
    /// The directory a new path goes in, or the path being renamed or copied.
    pub at: String,
    pub field: Field,
}

impl Naming {
    /// A name asked for at this path, prefilled with what it starts from.
    pub fn new(asked: Asked, at: &str, from: &str) -> Self {
        let mut field = Field::default();
        field.set(from);
        Self {
            asked,
            at: at.to_string(),
            field,
        }
    }

    /// What the name says, with nothing around it.
    pub fn named(&self) -> &str {
        self.field.text().trim()
    }
}

/// What a name typed in the tree is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    File,
    Folder,
    Rename,
    Copy,
}

/// Which of the sidebar's three lists is up.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    #[default]
    Files,
    Commits,
    Notes,
}

impl Pane {
    /// Every pane, in the order the strip shows them.
    pub const ALL: [Pane; 3] = [Pane::Files, Pane::Commits, Pane::Notes];

    pub fn label(self) -> &'static str {
        match self {
            Pane::Files => "changed",
            Pane::Commits => "commits",
            Pane::Notes => "notes",
        }
    }
}

/// What the sidebar's bar narrows by: a path, some text, or both.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Bar {
    pub path: Field,
    pub text: Field,
    /// Which of the two the keyboard is in, when it is in either.
    pub typing: Option<Term>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Term {
    #[default]
    Path,
    Text,
}

impl Term {
    /// Both terms, in the order the bar stacks them.
    pub const ALL: [Term; 2] = [Term::Path, Term::Text];

    pub fn label(self) -> &'static str {
        match self {
            Term::Path => "path",
            Term::Text => "text",
        }
    }
}

impl Bar {
    /// The bar open on one term, the other kept, this one's text cleared.
    pub fn open(&mut self, term: Term) {
        self.typing = Some(term);
        self.of(term).clear();
    }

    /// The keyboard in one term, leaving what it holds to be edited.
    pub fn focus(&mut self, term: Term) {
        self.typing = Some(term);
        self.of(term).end();
    }

    pub fn shut(&mut self) {
        *self = Self::default();
    }

    pub fn of(&mut self, term: Term) -> &mut Field {
        match term {
            Term::Path => &mut self.path,
            Term::Text => &mut self.text,
        }
    }

    /// Whether a search is on.
    pub fn in_use(&self) -> bool {
        self.typing.is_some() || !self.path.is_empty() || !self.text.is_empty()
    }

    /// Whether the results are lines rather than files.
    pub fn greps(&self) -> bool {
        !self.text.is_empty()
    }
}

impl SessionUi {
    pub fn face(&self) -> Face {
        match self.tab {
            Tab::Files => Face::File,
            _ => Face::Stream(self.view),
        }
    }

    /// How far the surface the tab shows is scrolled.
    pub fn scroll(&self) -> f32 {
        match self.face() {
            Face::File => self.file,
            Face::Stream(_) => self.diff,
        }
    }

    pub fn scroll_mut(&mut self) -> &mut f32 {
        match self.face() {
            Face::File => &mut self.file,
            Face::Stream(_) => &mut self.diff,
        }
    }

    /// How far the surface the tab shows is scrolled sideways.
    pub fn across(&self) -> f32 {
        match self.face() {
            Face::File => self.file_across,
            Face::Stream(_) => self.diff_across,
        }
    }

    pub fn across_mut(&mut self) -> &mut f32 {
        match self.face() {
            Face::File => &mut self.file_across,
            Face::Stream(_) => &mut self.diff_across,
        }
    }

    /// Whether a bar has the keyboard.
    pub fn typing(&self) -> bool {
        self.bar.typing.is_some()
            || self.find.as_ref().is_some_and(|find| find.typing)
            || self.noting.is_some()
    }

    /// Whether the sidebar stands beside the workspace right now.
    pub fn sidebar(&self) -> bool {
        self.tab.has_sidebar() && !self.folded
    }

    /// Whether the commit box stands under the sidebar: whenever the sidebar does.
    pub fn commits(&self) -> bool {
        self.sidebar()
    }
}
