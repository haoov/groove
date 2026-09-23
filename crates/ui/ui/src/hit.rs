//! What the pointer can reach. A view registers a rect and what it means; a click
//! resolves to the last one registered over that point.

mod note;

pub use note::NoteButton;

use groove_gfx::Rect;
use groove_types::{SessionId, WorktreeId};

use groove_types::DiffView;

use crate::layout::Edge;
use crate::views::session::{Tab, Term};

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
    /// What the change is read against.
    Mode(groove_types::DiffMode),
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
    /// The rail's own first row, which opens the board.
    Board,
    /// A task on the board, which a click opens the session for.
    Task(String),
    /// A task's place in the plan, which a drag moves.
    Place(groove_types::ExternalId),
    /// What hands the source the hours the clock measured.
    LogHours(groove_types::ExternalId),
    /// The timeline's own bar, which folds it away.
    Timeline,
    /// A directory of the explorer, which a click opens or shuts.
    Dir(String),
    /// Which files the sidebar lists.
    Scope(crate::views::session::Scope),
    /// Which of the sidebar's lists is up.
    Pane(crate::views::session::Pane),
    /// One note of the sidebar's list, which a click opens the line of.
    NoteAt(usize),
    /// One commit of the sidebar's list, which a click shows the change of.
    Commit(String),
    /// What leaves the commit and shows the working tree again.
    Working,
    /// The feed's own heading, which folds it away.
    Feed,
    /// A line of the feed, and the session it belongs to.
    FeedLine(SessionId),
    /// The write the agent asked for, taken or refused.
    Approve(groove_types::ApprovalId),
    Refuse(groove_types::ApprovalId),
    /// Which sessions the feed shows.
    FeedScope,
    /// One MR of the review column, by its project and its number.
    Review(String, u64),
    /// What finishes the task a session works, and what opens its other actions.
    Finish(groove_types::SessionId),
    /// What reads the selected worktree's MR again.
    Refresh,
    TaskActions(groove_types::SessionId),
    /// One task's bar on the timeline, which names it under the pointer.
    Bar(String),
    /// The board's filter, and one row it offers.
    Filter,
    Offer(usize),
    /// What starts a task, at the right of the board's header.
    AddTask,
    /// A live item's twisty, which shows its worktrees.
    Unfold(SessionId),
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
    /// The lines standing above them.
    Pinned,
    /// The column holding the whole change.
    Map,
    /// What marks a file read, at the end of its head row.
    Read(String),
    /// The row a file starts on, which folds it.
    Head(String),
    /// One line a search across the worktree found, by its place in the list.
    Found(usize),
    /// The row naming a file the search found lines in, which folds them.
    FoundIn(String),
    /// One term of the sidebar's search bar.
    Term(Term),
    /// The bar over the rows while a search of them is live.
    Finding,
    /// One of the three views of the open file.
    View(DiffView),
    /// The agent's pane.
    Agent,
    /// One button of a note's own row.
    Note(groove_types::NoteOrigin, NoteButton),
}

impl Target {
    /// What the pointer says over it: a row is a pointer, and these are not.
    fn cursor(&self) -> Cursor {
        match self {
            Target::Term(_) | Target::Finding | Target::Filter | Target::Message | Target::Code => {
                Cursor::Text
            }
            Target::Agent | Target::Pinned | Target::Bar(_) | Target::Palette => Cursor::Default,
            Target::Map | Target::Place(_) => Cursor::RowResize,
            Target::Split(edge) => match edge.upright() {
                true => Cursor::ColResize,
                false => Cursor::RowResize,
            },
            _ => Cursor::Pointer,
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
    /// How far down it was drawn.
    pub scroll: f32,
}

/// A column that scrolls on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scroller {
    Rail,
    Files,
    Code,
    Overview,
    /// The rail's own log.
    Feed,
    /// One of the board's columns.
    Column(u8),
}

impl Scroller {
    /// Its place among the extents one frame records.
    fn at(self) -> usize {
        match self {
            Scroller::Rail => 0,
            Scroller::Files => 1,
            Scroller::Code => 2,
            Scroller::Overview => 3,
            Scroller::Feed => 4,
            Scroller::Column(which) => 5 + which as usize,
        }
    }
}

/// Where everything was drawn this frame.
#[derive(Debug, Default)]
pub struct Hits {
    regions: Vec<(Rect, Target)>,
    /// The rows the code surface drew.
    shown: std::ops::Range<usize>,
    /// How far each column can scroll, one per `Scroller`.
    extents: [f32; 8],
    chars: Chars,
}

impl Hits {
    pub fn push(&mut self, rect: Rect, target: Target) {
        self.regions.push((rect, target));
    }

    /// How far a column could scroll when it was drawn.
    pub fn scrolls(&mut self, which: Scroller, extent: f32) {
        let at = which.at();
        self.extents[at] = self.extents[at].max(extent);
    }

    pub fn extent(&self, which: Scroller) -> f32 {
        self.extents[which.at()]
    }

    /// Which rows of the whole change the surface drew.
    pub fn showing(&mut self, rows: std::ops::Range<usize>) {
        self.shown = rows;
    }

    pub fn shown(&self) -> std::ops::Range<usize> {
        self.shown.clone()
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
