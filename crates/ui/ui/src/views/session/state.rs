//! What the session surface remembers between frames.

use groove_types::DiffView;

pub use super::tab::{Face, Tab};
pub use noting::{Noting, Writing};

mod noting;

use super::find::Finding;
use groove_ui_kit::widgets::Field;

/// What the session surface remembers between frames.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SessionUi {
    pub tab: Tab,
    /// The tab shown when last settled.
    pub settled: Tab,
    /// The tab to show once the repo asked for is selected and read, and its commit shown when one.
    pub going: Option<(Tab, Option<String>)>,
    /// The session and the worktree selected when last settled.
    pub seen: (
        Option<groove_types::SessionId>,
        Option<groove_types::WorktreeId>,
    ),
    /// How far the sidebar's list is scrolled, in pixels.
    pub files: f32,
    /// The user folded the sidebar away.
    pub folded: bool,
    /// The manual section stands open under the tab.
    pub manual: bool,
    /// The workspace is folded into its strip, the agent wide.
    pub stowed: bool,
    /// The session is a routine's: its agent pane alone, the window wide.
    pub alone: bool,
    /// A commit is shown in place of the working tree.
    pub reading: bool,
    /// How far the stream is scrolled, in pixels.
    pub diff: f32,
    /// How far the active file is scrolled, in pixels.
    pub file: f32,
    /// How far the stream's text and the file's are scrolled sideways, in pixels.
    pub diff_across: f32,
    pub file_across: f32,
    /// The caret the file view last scrolled to.
    pub followed: Option<groove_types::Caret>,
    /// The landing of a definition the view last centred on.
    pub centred: u64,
    /// The completion ask the list's chosen row belongs to, and that row.
    pub chosen: (u64, usize),
    /// The line the caret rests on, and the frame tick it came to rest at.
    pub rest: Option<(crate::views::session::diff::Spot, u64)>,
    /// The spot of an open file the pointer stands on, and the one it last came to rest on.
    pub pointed: Option<crate::views::session::diff::Pointed>,
    pub pointer_rest: Option<(crate::views::session::diff::Pointed, u64)>,
    /// Where the caret stood at which revision and since when, and the word once it rested.
    pub word_rest: Option<((crate::views::session::diff::Pointed, u64), u64)>,
    pub word: Option<String>,
    /// The blocks folded in each file, by the line that opens them.
    pub folds: std::collections::BTreeMap<String, crate::views::session::diff::blocks::Folds>,
    /// How far the overview is scrolled, in pixels.
    pub overview: f32,
    /// Which of the two views the stream is drawn in.
    pub view: DiffView,
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
    /// The resolved notes opened out of their fold.
    pub unfolded: Vec<groove_types::NoteOrigin>,
    /// Which notes the sidebar's list shows.
    pub showing: Showing,
    pub resources: crate::views::session::resources::ResourcesUi,
    /// The Resources tab of each other session, as it was left.
    pub resources_of: std::collections::BTreeMap<
        groove_types::SessionId,
        crate::views::session::resources::ResourcesUi,
    >,
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

    pub fn mark(self) -> groove_ui_kit::base::mark::Mark {
        use groove_ui_kit::base::mark::Mark;
        match self {
            Pane::Files => Mark::List,
            Pane::Commits => Mark::Commits,
            Pane::Notes => Mark::Note,
        }
    }
}

/// Which of the session's notes the sidebar lists.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Showing {
    #[default]
    Open,
    Resolved,
    All,
}

impl Showing {
    pub const ALL: [Showing; 3] = [Showing::Open, Showing::Resolved, Showing::All];

    pub fn label(self) -> &'static str {
        match self {
            Showing::Open => "open",
            Showing::Resolved => "resolved",
            Showing::All => "all",
        }
    }

    /// Whether a note this resolved stands in the list.
    pub fn lists(self, resolved: bool) -> bool {
        match self {
            Showing::Open => !resolved,
            Showing::Resolved => resolved,
            Showing::All => true,
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
    /// Whether a bar has the keyboard.
    pub fn typing(&self) -> bool {
        self.bar.typing.is_some()
            || self.find.as_ref().is_some_and(|find| find.typing)
            || self.noting.is_some()
    }

    /// Whether the sidebar stands beside the workspace right now.
    pub fn sidebar(&self) -> bool {
        self.tab.has_sidebar() && !self.folded && !self.alone
    }
}

impl SessionUi {
    /// The tab gone to once what it waits for stands; nothing moves when it failed.
    pub fn arrive(&mut self, app: &groove_controllers::AppState) {
        let Some((tab, sha)) = &self.going else {
            return;
        };
        if app.workspace.revision.is_some() {
            return;
        }
        let shown = app.workspace.commit.as_ref().map(|one| &one.sha);
        if sha.is_none() || shown == sha.as_ref() {
            self.tab = *tab;
        }
        self.going = None;
    }
}
