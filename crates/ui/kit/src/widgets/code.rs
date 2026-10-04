//! A row of the code surface: a line of code, a banner across the width, or a file's head.

mod gutter;
mod paint;

use groove_gfx::{Color, Rect};
use groove_types::Highlight;

pub use self::gutter::{Block, Gutters, rule};

use crate::base::ctx::{App, Ctx};

/// One row of code; `T` is what a click on its blame opens.
pub struct Code<'a, T> {
    pub gutters: &'a [&'a str],
    pub text: &'a str,
    pub spans: &'a [Highlight],
    pub ground: Option<Color>,
    /// A bar at the row's left edge, for what a gutter number cannot say.
    pub mark: Option<Color>,
    pub banner: bool,
    pub head: bool,
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
    pub noted: bool,
    pub blame: Option<(&'a str, Option<T>)>,
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
        return paint::head(ctx, line, code);
    }
    if code.banner {
        return paint::banner(ctx, line, code.text);
    }
    paint::coded(ctx, line, code, gutter, across);
}

/// Where a head row carries the mark that says its file is read.
pub fn head_mark<A: App>(ctx: &Ctx<'_, A>, line: Rect) -> Rect {
    let size = ctx.tokens.icon;
    crate::shape::box_in(line, line.right() - ctx.tokens.md - size, size)
}
