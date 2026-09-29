//! Laid lines onto the frame: grounds, marks, pieces, and a hit on every link.

use groove_gfx::Rect;

use super::layout::{Ground, Laid, Line, Piece};
use super::parse::Bullet;
use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::square;

/// The lines from the top of `at` that `visible` shows; `link` names a link's target.
pub fn draw<A: App>(
    ctx: &mut Ctx<'_, A>,
    at: Rect,
    visible: Rect,
    laid: &Laid,
    link: &dyn Fn(&str) -> A::Target,
) {
    for line in &laid.lines {
        let row = Rect::new(at.x, at.y + line.y, at.w, line.h);
        if row.bottom() < visible.y || row.y > visible.bottom() {
            continue;
        }
        ground(ctx, row, line);
        if let Some((x, bullet)) = line.mark {
            marked(ctx, row, at.x + x, bullet);
        }
        ctx.clipped(row, |ctx| {
            for piece in &line.pieces {
                one(ctx, row, at.x, piece, link);
            }
        });
    }
}

fn ground<A: App>(ctx: &mut Ctx<'_, A>, row: Rect, line: &Line) {
    let hairline = ctx.tokens.hairline;
    match line.ground {
        Ground::Code => ctx.quad(row, ctx.styles.band()),
        Ground::Rule => {
            let middle = row.y + (row.h - hairline) / 2.0;
            ctx.quad(Rect::new(row.x, middle, row.w, hairline), ctx.styles.line());
        }
        Ground::Plain => {}
    }
    if line.quoted {
        let edge = Rect::new(row.x, row.y, hairline * 2.0, row.h);
        ctx.quad(edge, ctx.styles.color(Role::Ghost));
    }
}

pub(super) fn marked<A: App>(ctx: &mut Ctx<'_, A>, row: Rect, x: f32, bullet: Bullet) {
    let style = ctx.styles.prose(0, Role::Faint);
    let size = style.size;
    let (mark, role) = match bullet {
        Bullet::Dot => return ctx.text("•", x, row.y, row.h, style),
        Bullet::Number(n) => return ctx.text(&format!("{n}."), x, row.y, row.h, style),
        Bullet::Box(false) => (Mark::Unticked, Role::Faint),
        Bullet::Box(true) => (Mark::Ticked, Role::Ok),
    };
    let rect = square(Rect::new(x, row.y, size, row.h), size);
    ctx.icon(rect, mark, 0, ctx.styles.color(role));
}

fn one<A: App>(
    ctx: &mut Ctx<'_, A>,
    row: Rect,
    left: f32,
    piece: &Piece,
    link: &dyn Fn(&str) -> A::Target,
) {
    let x = left + piece.x;
    if piece.code {
        let pad = ctx.tokens.hairline * 2.0;
        let tall = piece.style.size + pad * 2.0;
        let wide = ctx.measure(piece.text.trim_end(), &piece.style) + pad * 2.0;
        let behind = Rect::new(x - pad, row.y + (row.h - tall) / 2.0, wide, tall);
        ctx.rounded(behind, ctx.styles.band(), ctx.tokens.corner);
    }
    ctx.text(&piece.text, x, row.y, row.h, piece.style);
    if let Some(url) = &piece.link {
        ctx.hit(Rect::new(x, row.y, piece.w, row.h), link(url));
    }
}
