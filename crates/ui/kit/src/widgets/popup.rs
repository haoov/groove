//! A panel of lines under a point: what the language servers say of one spot.

mod rows;

use groove_gfx::{Rect, TextStyle};
use groove_types::Highlight;

use self::rows::{Row, rows};
use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::box_in;
use crate::text::row;
use crate::widgets::{Corner, Material, panel_at};

/// A run of a line, set as code or as prose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    pub text: String,
    pub code: bool,
}

/// One line of what the panel says.
#[derive(Debug, Clone, PartialEq)]
pub enum Said {
    /// Prose in `role`, wrapped by words, after `mark` when it has one.
    Prose {
        pieces: Vec<Piece>,
        role: Role,
        mark: Option<Mark>,
    },
    /// A line of a code block, coloured by its spans, never wrapped.
    Code { text: String, spans: Vec<Highlight> },
    /// A rule across the panel.
    Rule,
    /// A little room between two lines.
    Gap,
}

/// The lines under `at`, the panel as wide as its widest row. Returns the panel.
pub fn popup<A: App>(ctx: &mut Ctx<'_, A>, at: (f32, f32), said: &[Said]) -> Rect {
    let pad = ctx.tokens.sm;
    let most = ctx.tokens.popup.min(ctx.window.w - ctx.tokens.xl) - pad * 2.0;
    let laid = rows(ctx, said, most);
    let wide = laid.iter().map(|one| one.width).fold(0.0, f32::max);
    let tall: f32 = laid.iter().map(|one| height(ctx, &one.row)).sum();
    let size = (wide + pad * 2.0, tall + pad * 2.0);
    let rect = panel_at(
        ctx,
        at,
        Corner::TopLeft,
        size,
        (ctx.styles.line(), Material::Glass),
    );
    let mut y = rect.y + pad;
    for one in &laid {
        let h = height(ctx, &one.row);
        let line = Rect::new(rect.x + pad, y, rect.w - pad * 2.0, h);
        draw(ctx, line, &one.row);
        y += h;
    }
    rect
}

fn height<A: App>(ctx: &Ctx<'_, A>, row: &Row) -> f32 {
    match row {
        Row::Rule => ctx.tokens.sm,
        Row::Gap => ctx.tokens.xs,
        _ => ctx.tokens.line,
    }
}

fn draw<A: App>(ctx: &mut Ctx<'_, A>, line: Rect, laid: &Row) {
    match laid {
        Row::Rule => {
            let thin = ctx.tokens.hairline;
            let at = Rect::new(line.x, line.y + (line.h - thin) / 2.0, line.w, thin);
            ctx.quad(at, ctx.styles.line());
        }
        Row::Gap => {}
        Row::Prose { pieces, role, mark } => {
            let mut x = line.x;
            if let Some(mark) = mark {
                let size = ctx.tokens.small;
                ctx.icon(box_in(line, x, size), *mark, 0, ctx.styles.color(*role));
                x += size + ctx.tokens.xs;
            }
            for piece in pieces {
                let style = styled(ctx, piece.code, *role);
                if piece.code {
                    grounded(ctx, (line, x), piece.text.trim_end(), style);
                }
                x = put(ctx, (line, x), &piece.text, style);
            }
        }
        Row::Code { text, spans } => coloured(ctx, line, text, spans),
    }
}

pub(super) fn styled<A: App>(ctx: &Ctx<'_, A>, code: bool, role: Role) -> TextStyle {
    match code {
        true => ctx.styles.prose_code(role),
        false => ctx.styles.body(role),
    }
}

/// The ground under a piece of inline code, as prose sets it.
fn grounded<A: App>(ctx: &mut Ctx<'_, A>, (line, x): (Rect, f32), text: &str, style: TextStyle) {
    let pad = ctx.tokens.hairline * 2.0;
    let tall = style.size + pad * 2.0;
    let wide = ctx.measure(text, &style) + pad * 2.0;
    let behind = Rect::new(x - pad, line.y + (line.h - tall) / 2.0, wide, tall);
    ctx.quad(behind, ctx.styles.glass_code_ground());
}

/// A code line in the pieces its spans cut it into.
fn coloured<A: App>(ctx: &mut Ctx<'_, A>, line: Rect, text: &str, spans: &[Highlight]) {
    let pad = ctx.tokens.xs;
    let ground = Rect::new(line.x - pad, line.y, line.w + pad * 2.0, line.h);
    ctx.quad(ground, ctx.styles.glass_code_ground());
    let plain = ctx.styles.prose_code(Role::Text);
    let (mut x, mut cut) = (line.x, 0);
    for span in spans {
        let (Some(before), Some(inside)) = (
            text.get(cut..span.range.start),
            text.get(span.range.clone()),
        ) else {
            break;
        };
        x = put(ctx, (line, x), before, plain);
        let mut style = plain;
        style.color = ctx.styles.syntax(span.capture);
        x = put(ctx, (line, x), inside, style);
        cut = span.range.end;
    }
    if let Some(rest) = text.get(cut..) {
        put(ctx, (line, x), rest, plain);
    }
}

/// `text` at `x` of `line`, clipped to it. Returns where the next piece starts.
fn put<A: App>(ctx: &mut Ctx<'_, A>, (line, x): (Rect, f32), text: &str, style: TextStyle) -> f32 {
    if text.is_empty() || x >= line.right() {
        return x;
    }
    let width = ctx.measure(text, &style);
    let at = Rect::new(x, line.y, (line.right() - x).max(0.0), line.h);
    row(ctx, at, 0.0, text, style);
    x + width
}
