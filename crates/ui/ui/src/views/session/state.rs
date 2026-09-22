//! What the session surface remembers between frames.

use groove_types::{Anchor, DiffView};

use super::find::Finding;
use crate::widget::Field;

/// Which tab of the workspace is up.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Overview,
    /// The file: its own text, or the change in it.
    File,
}

impl Tab {
    /// Every tab, in the order the strip shows them.
    pub const ALL: [Tab; 2] = [Tab::Overview, Tab::File];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Overview => "overview",
            Tab::File => "file",
        }
    }

    /// Whether the tab brings its own list beside the workspace.
    pub fn has_sidebar(self) -> bool {
        match self {
            Tab::Overview => false,
            Tab::File => true,
        }
    }
}

/// What the session surface remembers between frames.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SessionUi {
    pub tab: Tab,
    /// How far the sidebar's list is scrolled, in pixels.
    pub files: f32,
    /// The user folded the sidebar away.
    pub folded: bool,
    /// How far the open file is scrolled, in pixels.
    pub diff: f32,
    /// How far the overview is scrolled, in pixels.
    pub overview: f32,
    /// Which of the three views the open file is drawn in.
    pub view: DiffView,
    /// The keyboard is in the commit box.
    pub composing: bool,
    /// What the sidebar's search bar narrows by.
    pub bar: Bar,
    /// The files whose found lines are hidden under their own row.
    pub shut: std::collections::BTreeSet<String>,
    /// The bar over the rows, while a search of them is live.
    pub find: Option<Finding>,
    /// Which files the sidebar lists: the ones that changed, or the whole worktree.
    pub scope: Scope,
    /// Which of the sidebar's lists is up.
    pub pane: Pane,
    /// The explorer's own directories that stand open.
    pub opened: std::collections::BTreeSet<String>,
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
            Pane::Files => "files",
            Pane::Commits => "commits",
            Pane::Notes => "notes",
        }
    }
}

/// Which files the sidebar lists.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    #[default]
    Changed,
    All,
}

impl Scope {
    pub const ALL: [Scope; 2] = [Scope::Changed, Scope::All];

    pub fn label(self) -> &'static str {
        match self {
            Scope::Changed => "changed",
            Scope::All => "all",
        }
    }
}

/// What the sidebar's bar narrows by: a path, some text, or both at once. The path
/// says which files to look at, the text what to look for in them.
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
    /// The bar open on one term, keeping what the other holds. What that term held
    /// is spent, since a chord asks for a new one.
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

    /// Whether a search is on, so the bar stands on both its terms.
    pub fn in_use(&self) -> bool {
        self.typing.is_some() || !self.path.is_empty() || !self.text.is_empty()
    }

    /// Whether the results are lines rather than files.
    pub fn greps(&self) -> bool {
        !self.text.is_empty()
    }
}

impl SessionUi {
    /// Whether a bar has the keyboard, so no surface should draw its caret.
    pub fn typing(&self) -> bool {
        self.bar.typing.is_some()
            || self.find.as_ref().is_some_and(|find| find.typing)
            || self.noting.is_some()
    }

    /// Whether the sidebar stands beside the workspace right now.
    pub fn sidebar(&self) -> bool {
        self.tab.has_sidebar() && !self.folded
    }

    /// Whether the commit box stands under the sidebar, which only the files list has.
    pub fn commits(&self) -> bool {
        self.sidebar() && self.pane == Pane::Files
    }
}
