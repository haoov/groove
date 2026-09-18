//! What the pointer can reach. A view registers a rect and what it means; a click
//! resolves to the last one registered over that point.

use groove_gfx::Rect;
use groove_types::{SessionId, WorktreeId};

use groove_types::DiffView;

use crate::layout::Edge;
use crate::views::session::Tab;

/// Which of the header's pickers was asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picks {
    Repo,
    Branch,
}

/// What the pointer looks like over a region.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Cursor {
    #[default]
    Default,
    /// Over something a click acts on.
    Pointer,
    /// Over a boundary a drag moves sideways.
    ColResize,
    /// Over one a drag moves up and down.
    RowResize,
    /// Over code.
    Text,
}

/// A thing on screen the pointer can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A rail row.
    Session(SessionId),
    /// A tab of the workspace.
    Tab(Tab),
    /// One of the header's two pickers.
    Picker(Picks),
    /// An overview row.
    Worktree(WorktreeId),
    /// A palette row, by its place in the list.
    PaletteRow(usize),
    /// The palette's box; a click on it does nothing.
    Palette,
    /// A boundary between two columns.
    Split(Edge),
    /// What folds the sidebar away.
    Fold,
    /// A file's row in the sidebar.
    File(String),
    /// What a row offers while the pointer is on it.
    Stage(String),
    Unstage(String),
    /// The two answers to what is asked before a change is thrown away.
    Discard,
    Keep,
    /// What the commit box offers beyond committing.
    Actions,
    /// A row of the menu the right button opens.
    MenuRow(usize),
    /// The commit message, and what the box does now.
    Message,
    Do,
    /// The open file's rows.
    Code,
    /// One of the three views of the open file.
    View(DiffView),
    /// The agent's pane.
    Agent,
}

impl Target {
    /// What the pointer says over it.
    fn cursor(&self) -> Cursor {
        match self {
            Target::Session(_)
            | Target::Tab(_)
            | Target::Picker(_)
            | Target::Worktree(_)
            | Target::PaletteRow(_)
            | Target::Fold
            | Target::File(_)
            | Target::Stage(_)
            | Target::Unstage(_)
            | Target::Discard
            | Target::Keep
            | Target::Actions
            | Target::MenuRow(_)
            | Target::Do
            | Target::View(_) => Cursor::Pointer,
            Target::Message => Cursor::Text,
            Target::Code => Cursor::Text,
            Target::Agent => Cursor::Default,
            Target::Split(edge) => match edge.upright() {
                true => Cursor::ColResize,
                false => Cursor::RowResize,
            },
            Target::Palette => Cursor::Default,
        }
    }
}

/// How the code surface laid its characters out, as the last frame drew it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Chars {
    /// Where a line's text starts, past the gutters.
    pub left: f32,
    /// How wide one character of it is.
    pub advance: f32,
}

/// A column that scrolls on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scroller {
    Rail,
    Files,
    Code,
}

/// Where everything was drawn this frame.
#[derive(Debug, Default)]
pub struct Hits {
    regions: Vec<(Rect, Target)>,
    /// How far each column can scroll, one per `Scroller`.
    extents: [f32; 3],
    chars: Chars,
}

impl Hits {
    pub fn push(&mut self, rect: Rect, target: Target) {
        self.regions.push((rect, target));
    }

    /// How far a column could scroll when it was drawn.
    pub fn scrolls(&mut self, which: Scroller, extent: f32) {
        let at = which as usize;
        self.extents[at] = self.extents[at].max(extent);
    }

    pub fn extent(&self, which: Scroller) -> f32 {
        self.extents[which as usize]
    }

    /// Where the open file's characters went, for the click that follows.
    pub fn characters(&mut self, chars: Chars) {
        self.chars = chars;
    }

    pub fn chars(&self) -> Chars {
        self.chars
    }

    /// The last thing drawn over this point.
    pub fn at(&self, x: f32, y: f32) -> Option<Target> {
        self.topmost(x, y).cloned()
    }

    /// The pointer over this point.
    pub fn cursor_at(&self, x: f32, y: f32) -> Cursor {
        self.topmost(x, y).map(Target::cursor).unwrap_or_default()
    }

    fn topmost(&self, x: f32, y: f32) -> Option<&Target> {
        self.regions
            .iter()
            .rev()
            .find(|(rect, _)| rect.contains(x, y))
            .map(|(_, target)| target)
    }

    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// Where this target was drawn.
    pub fn rect_of(&self, target: &Target) -> Option<Rect> {
        self.regions
            .iter()
            .find(|(_, at)| at == target)
            .map(|(rect, _)| *rect)
    }
}
