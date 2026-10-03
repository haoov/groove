//! Blocks wrapped to a width: lines of measured pieces, and the height they stand.

use groove_gfx::TextStyle;

use super::parse::{Block, Bullet, Kind, Span};
use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;
use crate::base::tokens::LEADING;

/// A run of one style on a line, `x` from the prose's left edge.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub x: f32,
    pub w: f32,
    pub text: String,
    pub style: TextStyle,
    pub code: bool,
    pub link: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ground {
    Plain,
    Code,
    Rule,
}

/// One line, `y` from the prose's top; its mark at its own `x` from the left edge.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub y: f32,
    pub h: f32,
    pub pieces: Vec<Piece>,
    pub mark: Option<(f32, Bullet)>,
    pub ground: Ground,
    pub quoted: bool,
}

#[derive(Debug, Default)]
pub struct Laid {
    pub lines: Vec<Line>,
    pub height: f32,
}

pub fn laid<A: App>(ctx: &mut Ctx<'_, A>, blocks: &[Block], width: f32, role: Role) -> Laid {
    let mut laid = Laid::default();
    let mut before: Option<Kind> = None;
    for block in blocks {
        laid.height += gap(ctx, before, block.kind);
        let lines = match block.kind {
            Kind::Code => code(ctx, block, laid.height),
            Kind::Rule => vec![rule(ctx, laid.height)],
            _ => wrapped(ctx, block, (laid.height, width), role),
        };
        laid.height = lines.last().map_or(laid.height, |one| one.y + one.h);
        laid.lines.extend(lines);
        before = Some(block.kind);
    }
    laid
}

/// The room above a block, from the one before it.
fn gap<A: App>(ctx: &Ctx<'_, A>, before: Option<Kind>, now: Kind) -> f32 {
    match (before, now) {
        (None, _) => 0.0,
        (_, Kind::Heading(_)) => ctx.tokens.md,
        (Some(Kind::Item(_)), Kind::Item(_)) | (Some(Kind::Code), Kind::Code) => 0.0,
        _ => ctx.tokens.sm,
    }
}

/// How far a block's text stands in, and where its mark goes.
fn indents<A: App>(ctx: &Ctx<'_, A>, block: &Block) -> (f32, Option<(f32, Bullet)>) {
    let step = ctx.tokens.lg;
    let text = super::draw::indent(ctx, block.quoted, block.depth);
    match block.kind {
        Kind::Item(bullet) => (text, Some((text - step, bullet))),
        _ => (text, None),
    }
}

pub(super) fn style_of<A: App>(
    ctx: &Ctx<'_, A>,
    span: &Span,
    base: TextStyle,
    role: Role,
) -> TextStyle {
    match (span.code, &span.link) {
        (true, _) => ctx.styles.prose_code(role),
        (false, Some(_)) => {
            ctx.styles
                .emphasised(ctx.styles.prose(0, Role::Accent), span.strong, span.em)
        }
        (false, None) => ctx.styles.emphasised(base, span.strong, span.em),
    }
}

fn wrapped<A: App>(
    ctx: &mut Ctx<'_, A>,
    block: &Block,
    (top, width): (f32, f32),
    role: Role,
) -> Vec<Line> {
    let (level, role) = match (block.kind, block.quoted) {
        (Kind::Heading(level), _) => (level, Role::Text),
        (_, true) => (0, Role::Faint),
        _ => (0, role),
    };
    let base = ctx.styles.prose(level, role);
    let (indent, mark) = indents(ctx, block);
    let tall = (base.size * LEADING).round();
    let mut lines = vec![blank(top, tall, mark, block.quoted)];
    let mut x = indent;
    for span in &block.spans {
        let style = style_of(ctx, span, base, role);
        for word in words(&span.text) {
            let wide = ctx.measure(word.trim_end(), &style);
            let full = x > indent && x + wide > width;
            if word == "\n" || full {
                let y = lines.last().map_or(top, |one| one.y + one.h);
                lines.push(blank(y, tall, None, block.quoted));
                x = indent;
            }
            if word == "\n" || (x == indent && word.trim().is_empty()) {
                continue;
            }
            let step = ctx.measure(word, &style);
            if let Some(line) = lines.last_mut() {
                put(line, (x, step), word, style, span);
            }
            x += step;
        }
    }
    lines
}

fn blank(y: f32, h: f32, mark: Option<(f32, Bullet)>, quoted: bool) -> Line {
    Line {
        y,
        h,
        pieces: Vec::new(),
        mark,
        ground: Ground::Plain,
        quoted,
    }
}

/// A word onto the line: onto its last piece when that is of the same kind.
fn put(line: &mut Line, (x, w): (f32, f32), word: &str, style: TextStyle, span: &Span) {
    if let Some(last) = line.pieces.last_mut()
        && last.style == style
        && last.code == span.code
        && last.link == span.link
    {
        last.text.push_str(word);
        last.w += w;
        return;
    }
    line.pieces.push(Piece {
        x,
        w,
        text: word.to_string(),
        style,
        code: span.code,
        link: span.link.clone(),
    });
}

/// The text cut after each space, a line break its own word.
pub(super) fn words(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut from = 0;
    for (at, one) in text.char_indices() {
        if one == '\n' {
            out.extend((from < at).then(|| &text[from..at]));
            out.push("\n");
            from = at + 1;
        } else if one == ' ' {
            out.push(&text[from..=at]);
            from = at + 1;
        }
    }
    out.extend((from < text.len()).then(|| &text[from..]));
    out
}

fn code<A: App>(ctx: &mut Ctx<'_, A>, block: &Block, top: f32) -> Vec<Line> {
    let style = ctx.styles.prose_code(Role::Text);
    let (indent, _) = indents(ctx, block);
    let pad = ctx.tokens.sm;
    let tall = (style.size * LEADING).round();
    let mut y = top;
    let mut out = Vec::new();
    for span in &block.spans {
        let w = ctx.measure(&span.text, &style);
        let piece = Piece {
            x: indent + pad,
            w,
            text: span.text.clone(),
            style,
            code: false,
            link: None,
        };
        out.push(Line {
            pieces: vec![piece],
            ground: Ground::Code,
            ..blank(y, tall, None, block.quoted)
        });
        y += tall;
    }
    out
}

fn rule<A: App>(ctx: &Ctx<'_, A>, top: f32) -> Line {
    Line {
        ground: Ground::Rule,
        ..blank(top, ctx.tokens.md, None, false)
    }
}
