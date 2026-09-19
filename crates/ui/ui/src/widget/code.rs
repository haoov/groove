use std::ops::Range;

use groove_gfx::{Color, Rect, TextStyle};
use groove_types::Highlight;

use crate::ctx::Ctx;
use crate::hit::Chars;
use crate::mark::Mark;
use crate::style::Role;
use crate::tokens::Tokens;
use crate::widget::{row, ruled};

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
}

impl<'a> Line<'a> {
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
        }
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
}

/// The gutter a surface asks for: how many number columns, and their longest number.
#[derive(Debug, Clone, Copy)]
pub struct Gutters {
    pub cells: usize,
    pub digits: usize,
}

/// The gutter block, measured once for the surface.
#[derive(Debug, Clone, Copy)]
struct Block {
    cells: usize,
    width: f32,
}

impl Block {
    fn of(ctx: &mut Ctx, gutters: Gutters) -> Self {
        let numbers = ctx.styles.code(Role::Ghost);
        let widest = "0".repeat(gutters.digits);
        Self {
            cells: gutters.cells,
            width: ctx.measure(&widest, &numbers),
        }
    }

    /// Where a line's text starts, past every gutter and the hairline.
    fn content(&self, ctx: &Ctx, rect: Rect) -> f32 {
        if self.cells == 0 {
            return rect.x + ctx.tokens.md;
        }
        let small = ctx.tokens.sm;
        let block = (self.width + small) * self.cells as f32;
        rect.x + small + block + ctx.tokens.md
    }
}

/// Where a surface with these gutters puts its characters in `rect`.
/// The row at the top of a surface scrolled this far.
pub fn first(line: f32, scroll: f32) -> usize {
    (scroll / line).floor().max(0.0) as usize
}

pub fn chars_of(ctx: &mut Ctx, gutters: Gutters, rect: Rect, scroll: f32) -> Chars {
    let style = ctx.styles.code(Role::Text);
    Chars {
        left: Block::of(ctx, gutters).content(ctx, rect),
        advance: ctx.measure("M", &style),
        scroll,
    }
}

/// The rows `rect` has room for at `scroll`, among `total`.
pub fn visible(ctx: &Ctx, rect: Rect, total: usize, scroll: f32) -> Range<usize> {
    let height = ctx.tokens.line;
    let first = first(height, scroll).min(total);
    let shown = (rect.h / height).ceil() as usize + 1;
    first..(first + shown).min(total)
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
            draw(ctx, at, line, block);
            drawn.push(at);
            y += height;
        }
    });
    rule(ctx, rect, block);
    drawn
}

/// Where a head row carries the mark that says its file is read.
pub fn head_mark(ctx: &Ctx, line: Rect) -> Rect {
    let size = ctx.tokens.icon;
    let x = line.right() - ctx.tokens.md - size;
    Rect::new(x, line.y + (line.h - size) / 2.0, size, size)
}

/// How tall the rows stand together.
pub fn height(ctx: &Ctx, lines: usize) -> f32 {
    ctx.tokens.line * lines as f32
}

/// The row and the column a point lands on, counted from the first row.
pub fn code_at(
    tokens: &Tokens,
    chars: Chars,
    rect: Rect,
    point: (f32, f32),
) -> Option<(usize, usize)> {
    if !rect.contains(point.0, point.1) {
        return None;
    }
    let row = ((point.1 - rect.y + chars.scroll) / tokens.line)
        .floor()
        .max(0.0);
    let column = ((point.0 - chars.left) / chars.advance.max(1.0))
        .round()
        .max(0.0);
    Some((row as usize, column as usize))
}

/// The hairline the gutters end at, down the whole surface.
fn rule(ctx: &mut Ctx, rect: Rect, gutter: Block) {
    if gutter.cells == 0 {
        return;
    }
    let (color, thickness) = (ctx.styles.line(), ctx.tokens.hairline);
    let at = gutter.content(ctx, rect) - ctx.tokens.md;
    ctx.quad(Rect::new(at, rect.y, thickness, rect.h), color);
}

fn draw(ctx: &mut Ctx, line: Rect, code: &Line<'_>, gutter: Block) {
    if code.band {
        return band(ctx, line, code.text);
    }
    if code.head {
        return head(ctx, line, code);
    }
    if code.banner {
        return banner(ctx, line, code.text);
    }
    if let Some(ground) = code.ground {
        ctx.quad(line, ground);
    }
    if let Some(mark) = code.mark {
        let width = ctx.tokens.hairline * 2.0;
        ctx.quad(Rect::new(line.x, line.y, width, line.h), mark);
    }
    let numbers = ctx.styles.code(Role::Ghost);
    let mut at = line.x + ctx.tokens.sm;
    for text in code.gutters {
        let width = ctx.measure(text, &numbers);
        let cell = Rect::new(at + gutter.width - width, line.y, width, line.h);
        row(ctx, cell, 0.0, text, numbers);
        at += gutter.width + ctx.tokens.sm;
    }
    let at = gutter.content(ctx, line);
    let rect = Rect::new(at, line.y, line.right() - at, line.h);
    if let Some(color) = code.word {
        for (from, to) in code.words {
            shade(ctx, rect, code.text, (*from, *to), color);
        }
    }
    for (from, to) in code.found {
        marked(ctx, rect, code.text, (*from, *to), false);
    }
    if let Some(at) = code.standing {
        marked(ctx, rect, code.text, at, true);
    }
    if let Some(held) = code.held {
        holding(ctx, rect, code.text, held);
    }
    text(ctx, rect, code);
    if let Some(column) = code.caret {
        caret(ctx, rect, code.text, column);
        here(ctx, line);
    }
}

/// Where the caret is: a rule above the row and one below it, across the surface.
fn here(ctx: &mut Ctx, line: Rect) {
    let color = ctx.styles.here();
    ruled(ctx, line, color);
}

/// What a caret holds, under the text: a band over the characters, run out past the
/// line's end when the selection carries on to the next row.
fn holding(ctx: &mut Ctx, rect: Rect, text: &str, held: (usize, usize, bool)) {
    let (from, to, through) = held;
    let style = ctx.styles.code(Role::Text);
    let start = upto(ctx, text, from, &style);
    let mut end = upto(ctx, text, to, &style);
    if through {
        end += ctx.measure("M", &style);
    }
    let color = ctx.styles.held();
    ctx.quad(
        Rect::new(rect.x + start, rect.y, (end - start).max(1.0), rect.h),
        color,
    );
}

/// What a search found, under the text. The one it stands on reads as a selection,
/// which is what it is wherever a caret can hold it.
fn marked(ctx: &mut Ctx, rect: Rect, text: &str, at: (usize, usize), standing: bool) {
    let color = match standing {
        true => ctx.styles.held(),
        false => ctx.styles.found(),
    };
    shade(ctx, rect, text, at, color);
}

/// A band of `color` under the columns `at` covers.
fn shade(ctx: &mut Ctx, rect: Rect, text: &str, at: (usize, usize), color: Color) {
    let style = ctx.styles.code(Role::Text);
    let start = upto(ctx, text, at.0, &style);
    let end = upto(ctx, text, at.1, &style);
    ctx.quad(
        Rect::new(rect.x + start, rect.y, (end - start).max(1.0), rect.h),
        color,
    );
}

/// How wide the first `column` characters are.
fn upto(ctx: &mut Ctx, text: &str, column: usize, style: &TextStyle) -> f32 {
    let before: String = text.chars().take(column).collect();
    ctx.measure(&before, style)
}

/// The caret: a bar at the column, placed through the text before it.
fn caret(ctx: &mut Ctx, rect: Rect, text: &str, column: usize) {
    let style = ctx.styles.code(Role::Text);
    let at = rect.x + upto(ctx, text, column, &style);
    let (width, color) = (ctx.tokens.hairline * 2.0, ctx.styles.caret());
    ctx.quad(Rect::new(at, rect.y, width, rect.h), color);
}

/// A row across the width: its own ground, its text in the middle.
fn banner(ctx: &mut Ctx, line: Rect, text: &str) {
    let (panel, style) = (ctx.styles.panel(), ctx.styles.code(Role::Ghost));
    ctx.quad(line, panel);
    let width = ctx.measure(text, &style);
    let at = line.x + (line.w - width) / 2.0;
    row(ctx, Rect::new(at, line.y, width, line.h), 0.0, text, style);
}

/// A row naming a directory: its own ground, its path faint at the margin.
fn band(ctx: &mut Ctx, line: Rect, text: &str) {
    let (ground, style) = (ctx.styles.inner(), ctx.styles.small(Role::Faint));
    ctx.quad(line, ground);
    row(ctx, line, ctx.tokens.md, text, style);
}

/// A row naming a file: its own ground, a caret for its rows, its name.
fn head(ctx: &mut Ctx, line: Rect, code: &Line<'_>) {
    let role = match code.read {
        true => Role::Faint,
        false => Role::Text,
    };
    let (ground, style) = (ctx.styles.raised(), ctx.styles.label(role));
    ctx.quad(line, ground);
    let size = ctx.tokens.icon;
    let caret = Rect::new(
        line.x + ctx.tokens.xs,
        line.y + (line.h - size) / 2.0,
        size,
        size,
    );
    let turn = match code.folded {
        true => Mark::RIGHTWARDS,
        false => 0,
    };
    ctx.icon(caret, Mark::Down, turn, ctx.styles.color(Role::Faint));
    let at = caret.right() - line.x + ctx.tokens.xs;
    row(ctx, line, at, code.text, style);
    box_(ctx, head_mark(ctx, line), code.read);
}

/// The box that says whether a file is read, ticked once it is.
fn box_(ctx: &mut Ctx, rect: Rect, read: bool) {
    let inset = ctx.tokens.xs / 2.0;
    let square = Rect::new(
        rect.x + inset,
        rect.y + inset,
        rect.w - inset * 2.0,
        rect.h - inset * 2.0,
    );
    let role = match read {
        true => Role::Text,
        false => Role::Faint,
    };
    ctx.border(square, ctx.styles.color(role));
    if read {
        ctx.icon(rect, Mark::Read, 0, ctx.styles.color(role));
    }
}

/// The line's text, in the pieces its spans cut it into.
fn text(ctx: &mut Ctx, rect: Rect, code: &Line<'_>) {
    let plain = ctx.styles.code(Role::Text);
    if code.spans.is_empty() {
        return row(ctx, rect, 0.0, code.text, plain);
    }
    let mut at = rect.x;
    let mut cut = 0;
    for span in code.spans {
        let Some(before) = code.text.get(cut..span.range.start) else {
            break;
        };
        at = piece(ctx, rect, at, before, plain);
        let Some(inside) = code.text.get(span.range.clone()) else {
            break;
        };
        let mut style = plain;
        style.color = ctx.styles.syntax(span.capture);
        at = piece(ctx, rect, at, inside, style);
        cut = span.range.end;
    }
    if let Some(rest) = code.text.get(cut..) {
        piece(ctx, rect, at, rest, plain);
    }
}

/// One piece of a line at `x`. Returns where the next one starts.
fn piece(ctx: &mut Ctx, rect: Rect, x: f32, text: &str, style: TextStyle) -> f32 {
    if text.is_empty() {
        return x;
    }
    let width = ctx.measure(text, &style);
    row(ctx, Rect::new(x, rect.y, width, rect.h), 0.0, text, style);
    x + width
}
