//! A row of the code surface: a line of code, a banner across the width, or a file's head.

mod faults;
mod gutter;
mod head;
mod outline;
mod paint;

use groove_gfx::{Color, Rect};
use groove_types::{Highlight, Severity};

pub use self::gutter::{Block, Gutters};

use crate::base::ctx::{App, Ctx};

/// What a line's notes say in its marks' column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    /// How many open notes stand on it.
    Open(usize),
    /// Only resolved ones.
    Resolved,
}

/// One row of code; `T` is what a click on its blame opens.
pub struct Code<'a, T> {
    pub gutters: &'a [&'a str],
    pub text: &'a str,
    pub spans: &'a [Highlight],
    pub ground: Option<Color>,
    /// The colour of a changed row's numbers.
    pub gutter: Option<Color>,
    /// A bar at the row's left edge, for what a gutter number cannot say.
    pub mark: Option<Color>,
    pub banner: bool,
    pub head: bool,
    /// Which of its head's rows this one is, and how many the head takes.
    pub part: (usize, usize),
    /// What a head's file added and removed, and its errors and warnings.
    pub counts: Option<(u32, u32)>,
    pub tally: Option<(u32, u32)>,
    /// The notes a line carries, in its marks' column, and what a click on them opens.
    pub tile: Option<(Tile, Option<T>)>,
    /// A side of a split row with no line on it.
    pub empty: bool,
    pub folded: bool,
    pub read: bool,
    /// In characters.
    pub caret: Option<usize>,
    /// From, to, and whether it runs past the line.
    pub held: Option<(usize, usize, bool)>,
    /// In columns of the text.
    pub found: &'a [(usize, usize)],
    pub words: &'a [(usize, usize)],
    pub word: Option<Color>,
    pub standing: Option<(usize, usize)>,
    /// What a language server found, in columns of the text.
    pub faults: &'a [(usize, usize, Severity)],
    pub noted: bool,
    pub blame: Option<(&'a str, Option<T>)>,
    pub outline: Outline<'a, T>,
}

/// What the editor draws about a line besides its text.
pub struct Outline<'a, T> {
    /// Where the word the caret rests in stands on this line, in columns of the text.
    pub occurs: &'a [(usize, usize)],
    /// The columns a scope's guide runs down.
    pub guides: &'a [usize],
    /// The block the line opens: whether it is folded, the lines it hides, what its caret does.
    pub fold: Option<(bool, usize, T)>,
}

impl<T> Default for Outline<'_, T> {
    fn default() -> Self {
        Self {
            occurs: &[],
            guides: &[],
            fold: None,
        }
    }
}

/// The row in `line`, its text scrolled `across` to the left.
pub fn draw<A: App>(
    ctx: &mut Ctx<'_, A>,
    line: Rect,
    code: &Code<'_, A::Target>,
    gutter: Block,
    across: f32,
) {
    if code.head {
        return head::head(ctx, line, code);
    }
    if code.banner {
        return paint::banner(ctx, line, code.text);
    }
    paint::coded(ctx, line, code, gutter, across);
}

/// The whole head a head row is part `at` of, `rows` rows tall.
pub fn head_of(line: Rect, (at, rows): (usize, usize)) -> Rect {
    Rect {
        y: line.y - at as f32 * line.h,
        h: line.h * rows.max(1) as f32,
        ..line
    }
}

/// Where a head row carries the mark that says its file is read.
pub fn head_mark<A: App>(ctx: &Ctx<'_, A>, line: Rect) -> Rect {
    let size = ctx.tokens.icon;
    crate::shape::box_in(line, line.right() - ctx.tokens.md - size, size)
}
