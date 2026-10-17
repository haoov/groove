//! The list a completion offers under the caret: a label and its detail a row, the chosen one raised.

use groove_gfx::Rect;

use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;
use crate::base::tokens::COMPLETION_ROWS;
use crate::text::row;
use crate::widgets::{Corner, panel_at};

/// One row of the list.
pub struct Offer<'a> {
    pub label: &'a str,
    pub detail: &'a str,
}

/// The rows around `chosen` with their top-left at `at`. Returns the panel.
pub fn completions<A: App>(
    ctx: &mut Ctx<'_, A>,
    at: (f32, f32),
    offers: &[Offer],
    chosen: usize,
) -> Option<Rect> {
    let shown = COMPLETION_ROWS.min(offers.len());
    let first = chosen
        .saturating_sub(shown.checked_sub(1)?)
        .min(offers.len() - shown);
    let window = &offers[first..first + shown];
    let (label, detail) = (ctx.styles.code(Role::Text), ctx.styles.code(Role::Faint));
    let (pad, gap) = (ctx.tokens.sm, ctx.tokens.md);
    let mut widest = |pick: for<'b> fn(&'b Offer<'b>) -> &'b str, style| {
        window
            .iter()
            .map(|one| ctx.measure(pick(one), &style))
            .fold(0.0, f32::max)
    };
    let labels = widest(|one| one.label, label);
    let details = widest(|one| one.detail, detail);
    let after = if details > 0.0 { gap + details } else { 0.0 };
    let most = ctx.tokens.popup.min(ctx.window.w - ctx.tokens.xl);
    let wide = (pad * 2.0 + labels + after).min(most);
    let size = (wide, ctx.tokens.row * shown as f32);
    let rect = panel_at(ctx, at, Corner::TopLeft, size, ctx.styles.line());
    for (index, one) in window.iter().enumerate() {
        let y = rect.y + ctx.tokens.row * index as f32;
        let line = Rect::new(rect.x, y, rect.w, ctx.tokens.row);
        if first + index == chosen {
            ctx.quad(line, ctx.styles.raised());
        }
        row(
            ctx,
            Rect::new(line.x, y, line.w - pad, line.h),
            pad,
            one.label,
            label,
        );
        let x = rect.x + pad + labels + gap;
        let room = Rect::new(x, y, (rect.right() - pad - x).max(0.0), line.h);
        row(ctx, room, 0.0, one.detail, detail);
    }
    Some(rect)
}
