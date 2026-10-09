//! What the pointer can reach: the last rect registered over a point wins.

mod note;
mod target;

pub use note::NoteButton;
pub use target::Target;

use groove_gfx::Rect;

/// Which of the header's pickers was asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picks {
    Repo,
    Branch,
    /// The Resources tab's contexts, and its namespaces.
    Contexts,
    Namespaces,
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
    /// The code surface's text, sideways.
    Across,
    Overview,
    /// The rail's own log.
    Feed,
    Settings,
    /// The Resources tab's list, or the object described.
    Resources,
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
            Scroller::Settings => 5,
            Scroller::Across => 6,
            Scroller::Resources => 7,
            Scroller::Column(which) => OWN + which as usize,
        }
    }
}

/// The scrollers ahead of the board's columns.
const OWN: usize = 8;

/// Where everything was drawn this frame.
#[derive(Debug, Default)]
pub struct Hits {
    regions: Vec<(Rect, Target)>,
    /// The rows the code surface drew.
    shown: std::ops::Range<usize>,
    wrap: usize,
    /// How far each column can scroll, one per `Scroller`.
    extents: [f32; OWN + crate::views::board::List::ALL.len()],
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

    /// How many characters a note row holds, as this frame measured it.
    pub fn wraps(&mut self, cols: usize) {
        self.wrap = cols;
    }

    pub fn wrap(&self) -> usize {
        self.wrap
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

    /// Where this target was drawn.
    pub fn rect_of(&self, target: &Target) -> Option<Rect> {
        self.regions
            .iter()
            .find(|(_, at)| at == target)
            .map(|(rect, _)| *rect)
    }
}
