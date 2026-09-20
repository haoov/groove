//! What the session surface remembers between frames.

use groove_types::DiffView;

use super::find::Finding;
use crate::widget::Field;

/// Which tab of the workspace is up.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Overview,
    Diff,
}

impl Tab {
    /// Every tab, in the order the strip shows them.
    pub const ALL: [Tab; 2] = [Tab::Overview, Tab::Diff];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Overview => "overview",
            Tab::Diff => "diff",
        }
    }

    /// Whether the tab brings its own list beside the workspace.
    pub fn has_sidebar(self) -> bool {
        match self {
            Tab::Overview => false,
            Tab::Diff => true,
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

    /// Whether the results are lines rather than files.
    pub fn greps(&self) -> bool {
        !self.text.is_empty()
    }
}

impl SessionUi {
    /// Whether a bar has the keyboard, so no surface should draw its caret.
    pub fn typing(&self) -> bool {
        self.bar.typing.is_some() || self.find.as_ref().is_some_and(|find| find.typing)
    }

    /// Whether the sidebar stands beside the workspace right now.
    pub fn sidebar(&self) -> bool {
        self.tab.has_sidebar() && !self.folded
    }
}
