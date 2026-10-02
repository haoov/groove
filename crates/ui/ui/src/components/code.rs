//! The code surface: a row's parts, and the gutters every row shares.

mod gutter;
mod place;
mod row;

use groove_gfx::{Color, Rect};
use groove_types::Highlight;

pub use self::place::{across_extent, chars_of, code_at, first, height, visible};

use self::gutter::{Block, rule};
use self::row::draw;
use crate::ctx::Ctx;

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
}

/// The buttons a note's last row carries, and the note they act on.
#[derive(Debug, Clone)]
pub struct Acting {
    pub origin: groove_types::NoteOrigin,
    pub resolved: bool,
    /// The MR holds it.
    pub thread: bool,
    /// It can be posted.
    pub post: bool,
    /// The one of them the pointer stands on.
    pub hovered: Option<crate::hit::NoteButton>,
}

/// One row of code: what its gutters say, its text, and the colour over it.
pub struct Line<'a> {
    pub gutters: &'a [&'a str],
    pub text: &'a str,
    pub spans: &'a [Highlight],
    pub ground: Option<Color>,
    /// A bar at the row's left edge, for what a gutter number cannot say.
    pub mark: Option<Color>,
    /// A row across the whole width with no gutters: a gap, a note.
    pub banner: bool,
    /// A row that names a file, at the head of its own rows.
    pub head: bool,
    /// A row that names the directory the files under it share.
    pub band: bool,
    /// A head row that hides its file's rows.
    pub folded: bool,
    /// A head row whose file has been read.
    pub read: bool,
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
    /// One row of a note left on the line above it.
    pub said: Option<Noted<'a>>,
    /// The buttons under what a note says.
    pub acting: Option<Acting>,
    /// A row of code a note stands on.
    pub noted: bool,
    /// Who last changed the line, after its end, and the commit a click opens.
    pub blame: Option<(&'a str, Option<&'a str>)>,
}

impl<'a> Line<'a> {
    /// Whether the row carries gutters; a row that names a file or directory has none.
    pub fn numbered(&self) -> bool {
        !self.band && !self.head && !self.banner && self.said.is_none() && self.acting.is_none()
    }

    pub fn new(text: &'a str) -> Self {
        Self {
            gutters: &[],
            text,
            spans: &[],
            ground: None,
            mark: None,
            banner: false,
            head: false,
            band: false,
            folded: false,
            read: false,
            caret: None,
            held: None,
            found: &[],
            words: &[],
            word: None,
            standing: None,
            said: None,
            acting: None,
            noted: false,
            blame: None,
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

    /// The row a directory starts on.
    pub fn band(text: &'a str) -> Self {
        Self {
            band: true,
            ..Self::new(text)
        }
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
}

/// The rows a surface draws: the window it built, and the gutter they share.
pub struct Rows<'a> {
    pub lines: &'a [Line<'a>],
    /// Where the first line sits among all the rows.
    pub first: usize,
    pub gutters: Gutters,
    /// How far the text is scrolled sideways, its gutters left in place.
    pub across: f32,
}

/// The gutter a surface asks for: how many number columns, and their longest number.
#[derive(Debug, Clone, Copy)]
pub struct Gutters {
    pub cells: usize,
    pub digits: usize,
}

/// Rows of code from the top of `rect`, scrolled by `scroll`, clipped to it.
pub fn code(ctx: &mut Ctx, rect: Rect, rows: Rows<'_>, scroll: f32) -> Vec<Rect> {
    let height = ctx.tokens.line;
    let block = Block::of(ctx, rows.gutters);
    let mut drawn = Vec::with_capacity(rows.lines.len());
    ctx.clipped(rect, |ctx| {
        let mut y = rect.y - scroll + rows.first as f32 * height;
        for line in rows.lines {
            let at = Rect::new(rect.x, y, rect.w, height);
            draw(ctx, at, line, block, rows.across);
            if line.numbered() {
                rule(ctx, at, block);
            }
            drawn.push(at);
            y += height;
        }
    });
    drawn
}

/// Where a head row carries the mark that says its file is read.
pub fn head_mark(ctx: &Ctx, line: Rect) -> Rect {
    let size = ctx.tokens.icon;
    let x = line.right() - ctx.tokens.md - size;
    Rect::new(x, line.y + (line.h - size) / 2.0, size, size)
}
