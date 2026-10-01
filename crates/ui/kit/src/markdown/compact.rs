//! Markdown on rows of one height: wrapped by characters, then drawn a row at a time.

use groove_gfx::Rect;

use super::draw::marked;
use super::layout::{style_of, words};
use super::parse::{Block, Bullet, Kind, Span, blocks};
use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;

/// One row: its spans, the mark it opens with, how deep it stands.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Row {
    pub spans: Vec<Span>,
    pub mark: Option<Bullet>,
    pub depth: usize,
    pub code: bool,
    pub quoted: bool,
}

/// `text` as rows of at most `cols` characters; a blank row between blocks.
pub fn compact(text: &str, cols: usize) -> Vec<Row> {
    let mut out: Vec<Row> = Vec::new();
    let mut before: Option<Kind> = None;
    for block in blocks(text) {
        let joined = matches!(
            (before, block.kind),
            (Some(Kind::Item(_)), Kind::Item(_)) | (Some(Kind::Code), Kind::Code)
        );
        if before.is_some() && !joined {
            out.push(Row::default());
        }
        match block.kind {
            Kind::Code => out.extend(block.spans.iter().map(|span| coded(&block, span))),
            Kind::Rule => {}
            _ => out.extend(wrapped(&block, cols)),
        }
        before = Some(block.kind);
    }
    if out.is_empty() {
        out.push(Row::default());
    }
    out
}

fn coded(block: &Block, span: &Span) -> Row {
    Row {
        spans: vec![span.clone()],
        code: true,
        ..first(block)
    }
}

/// The row a block opens with, before any words.
fn first(block: &Block) -> Row {
    Row {
        spans: Vec::new(),
        mark: match block.kind {
            Kind::Item(bullet) => Some(bullet),
            _ => None,
        },
        depth: block.depth,
        code: false,
        quoted: block.quoted,
    }
}

fn wrapped(block: &Block, cols: usize) -> Vec<Row> {
    let heading = matches!(block.kind, Kind::Heading(_));
    let room = cols.saturating_sub(block.depth * 2).max(1);
    let mut rows = vec![first(block)];
    let mut taken = 0;
    for span in &block.spans {
        for word in words(&span.text) {
            let size = word.trim_end().chars().count();
            if word == "\n" || (taken > 0 && taken + size > room) {
                rows.push(Row {
                    mark: None,
                    ..first(block)
                });
                taken = 0;
            }
            if word == "\n" || (taken == 0 && word.trim().is_empty()) {
                continue;
            }
            let piece = Span {
                text: word.to_string(),
                strong: span.strong || heading,
                ..span.clone()
            };
            if let Some(row) = rows.last_mut() {
                put(row, piece);
            }
            taken += word.chars().count();
        }
    }
    rows
}

fn put(row: &mut Row, span: Span) {
    if let Some(last) = row.spans.last_mut()
        && (last.strong, last.em, last.code, &last.link)
            == (span.strong, span.em, span.code, &span.link)
    {
        last.text.push_str(&span.text);
        return;
    }
    row.spans.push(span);
}

/// One row in `rect`, its words in `role`; `link` names a link's target.
pub fn row<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    one: &Row,
    role: Role,
    link: &dyn Fn(&str) -> A::Target,
) {
    let step = ctx.tokens.lg;
    let quote = if one.quoted { ctx.tokens.md } else { 0.0 };
    let mut x = rect.x + quote + step * one.depth as f32;
    if one.code {
        ctx.quad(
            Rect::new(x, rect.y, rect.right() - x, rect.h),
            ctx.styles.prose_code_ground(),
        );
        x += ctx.tokens.sm;
    }
    if one.quoted {
        let edge = Rect::new(rect.x, rect.y, ctx.tokens.hairline * 2.0, rect.h);
        ctx.quad(edge, ctx.styles.color(Role::Ghost));
    }
    if let Some(bullet) = one.mark {
        marked(ctx, rect, x - step, bullet);
    }
    let base = ctx.styles.body(if one.quoted { Role::Faint } else { role });
    ctx.clipped(rect, |ctx| {
        for span in &one.spans {
            let style = style_of(ctx, span, base, role);
            let wide = ctx.measure(&span.text, &style);
            ctx.text(&span.text, x, rect.y, rect.h, style);
            if let Some(url) = &span.link {
                ctx.hit(Rect::new(x, rect.y, wide, rect.h), link(url));
            }
            x += wide;
        }
    });
}
