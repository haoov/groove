//! A table's rows laid out: each column as wide as its widest cell, all shrunk to the width when
//! they do not fit, a cut cell elided.

use super::layout::{Ground, Line, Piece, blank, style_of};
use super::parse::{Block, Kind};
use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;
use crate::base::tokens::LEADING;
use crate::text::elide;

pub(super) fn laid<A: App>(
    ctx: &mut Ctx<'_, A>,
    rows: &[Block],
    (top, width): (f32, f32),
    role: Role,
) -> Vec<Line> {
    let base = ctx.styles.prose(0, role);
    let tall = (base.size * LEADING).round();
    let columns = columns(ctx, rows, width, role);
    let mut lines = Vec::new();
    for (at, row) in rows.iter().enumerate() {
        let head = matches!(row.kind, Kind::Row { head: true });
        let mut line = blank(top + at as f32 * tall, tall, None, row.quoted);
        if head {
            line.ground = Ground::Head;
        }
        for (cell, &(x, w)) in columns.iter().enumerate() {
            line.pieces
                .extend(pieces(ctx, row, cell, (x, w), (role, head)));
        }
        lines.push(line);
    }
    lines
}

/// Each column's left edge and width.
fn columns<A: App>(
    ctx: &mut Ctx<'_, A>,
    rows: &[Block],
    width: f32,
    role: Role,
) -> Vec<(f32, f32)> {
    let count = rows
        .iter()
        .flat_map(|row| row.spans.iter().map(|one| one.cell + 1))
        .max()
        .unwrap_or_default();
    let mut widest = vec![0.0_f32; count];
    for row in rows {
        let head = matches!(row.kind, Kind::Row { head: true });
        for (cell, wide) in widest.iter_mut().enumerate() {
            *wide = wide.max(natural(ctx, row, cell, (role, head)));
        }
    }
    let gap = ctx.tokens.lg;
    let gaps = gap * count.saturating_sub(1) as f32;
    let whole: f32 = widest.iter().sum();
    let scale = match whole + gaps > width && whole > 0.0 {
        true => ((width - gaps) / whole).max(0.0),
        false => 1.0,
    };
    let mut x = 0.0;
    widest
        .into_iter()
        .map(|wide| {
            let column = (x, wide * scale);
            x += wide * scale + gap;
            column
        })
        .collect()
}

fn natural<A: App>(
    ctx: &mut Ctx<'_, A>,
    row: &Block,
    cell: usize,
    (role, head): (Role, bool),
) -> f32 {
    let base = ctx.styles.prose(0, role);
    let spans = row.spans.iter().filter(|one| one.cell == cell);
    spans
        .map(|span| {
            let style = ctx
                .styles
                .emphasised(style_of(ctx, span, base, role), head, span.em);
            ctx.measure(&span.text, &style)
        })
        .sum()
}

/// One cell's spans from its column's left edge, the one that runs past its right edge elided.
fn pieces<A: App>(
    ctx: &mut Ctx<'_, A>,
    row: &Block,
    cell: usize,
    (x, w): (f32, f32),
    (role, head): (Role, bool),
) -> Vec<Piece> {
    let base = ctx.styles.prose(0, role);
    let (mut at, end) = (x, x + w);
    let mut out = Vec::new();
    for span in row.spans.iter().filter(|one| one.cell == cell) {
        let style = ctx
            .styles
            .emphasised(style_of(ctx, span, base, role), head, span.em);
        let text = span.text.replace('\n', " ");
        let wide = ctx.measure(&text, &style);
        let (text, wide) = match at + wide > end {
            true => {
                let cut = elide(ctx, &text, &style, end - at);
                let wide = ctx.measure(&cut, &style);
                (cut, wide)
            }
            false => (text, wide),
        };
        out.push(Piece {
            x: at,
            w: wide,
            text,
            style,
            code: span.code,
            link: span.link.clone(),
        });
        at += wide;
        if at >= end {
            break;
        }
    }
    out
}
