//! The code surface: a row's parts, and the gutters every row shares.

mod header;
mod place;
mod row;
mod rows;

use groove_gfx::Color;
use groove_types::{Highlight, Severity};

pub use self::header::{Composed, Field, Header, Noted, Room, label_role};
pub use self::place::{across_extent, chars_of, code_at, first, height, visible};
pub use self::rows::{Rows, code};
pub use groove_ui_kit::widgets::code::{Block, Gutters, Outline, Tile, head_mark, head_of};

/// The buttons a note's last row carries, and the note they act on.
#[derive(Debug, Clone)]
pub struct Acting {
    pub origin: groove_types::NoteOrigin,
    pub resolved: bool,
    /// The MR holds it.
    pub thread: bool,
    /// It can be posted.
    pub post: bool,
}

/// One row of code: what its gutters say, its text, and the colour over it.
pub struct Line<'a> {
    pub gutters: &'a [&'a str],
    pub text: &'a str,
    pub spans: &'a [Highlight],
    pub ground: Option<Color>,
    /// The gutter's own ground and its numbers' colour.
    pub gutter: Option<(Color, Color)>,
    /// A bar at the row's left edge, for what a gutter number cannot say.
    pub mark: Option<Color>,
    /// A row across the whole width with no gutters: a gap, a note.
    pub banner: bool,
    /// A row that names a file, at the head of its own rows.
    pub head: bool,
    /// A head row that hides its file's rows.
    pub folded: bool,
    /// A head row whose file has been read.
    pub read: bool,
    /// Which of its head's rows a head row is, and how many the head takes.
    pub part: (usize, usize),
    /// What a head row's file added and removed, and its errors and warnings.
    pub counts: Option<(u32, u32)>,
    pub tally: Option<(u32, u32)>,
    /// A row the one above it already draws over: a head's later rows.
    pub covered: bool,
    /// A side of a split row with no line on it.
    pub empty: bool,
    pub pad: Option<Room>,
    pub composed: Option<Composed>,
    /// The notes this line carries, and the resolved one a click on them opens.
    pub tile: Option<(Tile, Option<groove_types::NoteOrigin>)>,
    /// Where the caret sits on this row, in characters.
    pub caret: Option<usize>,
    /// What is held on this row: from, to, and whether it runs past the line.
    pub held: Option<(usize, usize, bool)>,
    /// What a search found on this row, in columns of its text.
    pub found: &'a [(usize, usize)],
    /// The columns the row it pairs with does not have, and the colour over them.
    pub words: &'a [(usize, usize)],
    pub word: Option<Color>,
    /// The one of them it stands on, drawn as a selection is.
    pub standing: Option<(usize, usize)>,
    /// What a language server found, in columns of its text.
    pub faults: &'a [(usize, usize, Severity)],
    /// One row of a note left on the line above it.
    pub said: Option<Noted<'a>>,
    /// The buttons under what a note says.
    pub acting: Option<Acting>,
    /// A row of code a note stands on.
    pub noted: bool,
    /// Who last changed the line, after its end, and the commit a click opens.
    pub blame: Option<(&'a str, Option<&'a str>)>,
    /// Its fold names the line it opens.
    pub outline: Outline<'a, usize>,
}

impl<'a> Line<'a> {
    /// Whether the row carries gutters; a row that names a file has none.
    pub fn numbered(&self) -> bool {
        !self.head
            && !self.banner
            && !self.covered
            && self.pad.is_none()
            && self.composed.is_none()
            && self.said.is_none()
            && self.acting.is_none()
    }

    pub fn composed(composed: Composed) -> Self {
        Self {
            composed: Some(composed),
            ..Self::new("")
        }
    }

    pub fn pad(room: Room) -> Self {
        Self {
            pad: Some(room),
            ..Self::new("")
        }
    }

    pub fn covered() -> Self {
        Self {
            covered: true,
            ..Self::new("")
        }
    }

    pub fn outline(mut self, outline: Outline<'a, usize>) -> Self {
        self.outline = outline;
        self
    }

    pub fn new(text: &'a str) -> Self {
        Self {
            gutters: &[],
            text,
            spans: &[],
            ground: None,
            gutter: None,
            mark: None,
            banner: false,
            head: false,
            folded: false,
            read: false,
            part: (0, 1),
            counts: None,
            tally: None,
            covered: false,
            empty: false,
            pad: None,
            composed: None,
            tile: None,
            caret: None,
            held: None,
            found: &[],
            words: &[],
            word: None,
            standing: None,
            faults: &[],
            said: None,
            acting: None,
            noted: false,
            blame: None,
            outline: Outline::default(),
        }
    }

    /// One row of a note, under the line it was left on.
    pub fn note(text: &'a str, said: Noted<'a>) -> Self {
        Self {
            said: Some(said),
            ..Self::new(text)
        }
    }

    /// The row of buttons under what a note says.
    pub fn acting(acting: Acting) -> Self {
        Self {
            acting: Some(acting),
            ..Self::new("")
        }
    }

    /// A row of code a note stands on.
    pub fn tile(mut self, tile: Option<(Tile, Option<groove_types::NoteOrigin>)>) -> Self {
        self.tile = tile;
        self
    }

    pub fn noted(mut self, noted: bool) -> Self {
        self.noted = noted;
        self
    }

    /// A row that spans the width, centred, on its own ground.
    pub fn banner(text: &'a str) -> Self {
        Self {
            banner: true,
            ..Self::new(text)
        }
    }

    /// The row a file starts on, naming it.
    pub fn head(text: &'a str) -> Self {
        Self {
            head: true,
            ..Self::new(text)
        }
    }

    pub fn folded(mut self, folded: bool) -> Self {
        self.folded = folded;
        self
    }

    pub fn read(mut self, read: bool) -> Self {
        self.read = read;
        self
    }

    pub fn part(mut self, part: (usize, usize)) -> Self {
        self.part = part;
        self
    }

    pub fn empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    pub fn counts(mut self, counts: Option<(u32, u32)>) -> Self {
        self.counts = counts;
        self
    }

    pub fn tally(mut self, tally: Option<(u32, u32)>) -> Self {
        self.tally = tally;
        self
    }

    pub fn gutters(mut self, gutters: &'a [&'a str]) -> Self {
        self.gutters = gutters;
        self
    }

    pub fn spans(mut self, spans: &'a [Highlight]) -> Self {
        self.spans = spans;
        self
    }

    pub fn ground(mut self, ground: Color) -> Self {
        self.ground = ground.into();
        self
    }

    pub fn gutter(mut self, gutter: Option<(Color, Color)>) -> Self {
        self.gutter = gutter;
        self
    }

    pub fn mark(mut self, mark: Option<Color>) -> Self {
        self.mark = mark;
        self
    }

    pub fn caret(mut self, caret: Option<usize>) -> Self {
        self.caret = caret;
        self
    }

    pub fn held(mut self, held: Option<(usize, usize, bool)>) -> Self {
        self.held = held;
        self
    }

    pub fn found(mut self, found: &'a [(usize, usize)]) -> Self {
        self.found = found;
        self
    }

    pub fn words(mut self, words: &'a [(usize, usize)], word: Option<Color>) -> Self {
        self.words = words;
        self.word = word;
        self
    }

    pub fn blame(mut self, blame: Option<(&'a str, Option<&'a str>)>) -> Self {
        self.blame = blame;
        self
    }

    pub fn standing(mut self, standing: Option<(usize, usize)>) -> Self {
        self.standing = standing;
        self
    }

    pub fn faults(mut self, faults: &'a [(usize, usize, Severity)]) -> Self {
        self.faults = faults;
        self
    }
}
