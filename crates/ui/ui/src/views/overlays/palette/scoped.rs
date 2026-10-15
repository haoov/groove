//! The pinned line under the palette's input: the session, repo and branch an action runs on.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};

use crate::ctx::Ctx;
use crate::palette::Palette;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::shape::hairline;
use groove_ui_kit::text::{elide, row};

/// The line's words: the session's title, then its selected repo and branch.
pub(super) fn scope(app: &AppState, palette: &Palette) -> Option<Vec<String>> {
    if palette.anchor.is_some() || palette.flow.is_some() {
        return None;
    }
    let open = app.session.selected()?;
    let mut out = vec![open.session.title.clone()];
    out.extend(open.selected_repo().map(|repo| repo.project.clone()));
    out.extend(open.selected_worktree().map(|one| one.branch.clone()));
    Some(out)
}

/// The line at the top of `rect`; returns the room under it.
pub(super) fn draw(ctx: &mut Ctx, rect: Rect, words: &[String]) -> Rect {
    let tall = Spec::default().height(ctx.tokens.row);
    let [line, rest] = column_in(rect, [tall, Spec::fill()]);
    let (faint, muted) = (ctx.styles.small(Role::Faint), ctx.styles.small(Role::Muted));
    let room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let gap = ctx.measure(" \u{b7} ", &faint);
    let tail: f32 = words
        .iter()
        .skip(1)
        .map(|one| gap + ctx.measure(one, &muted))
        .sum();
    let lead = ctx.measure("in ", &faint);
    let mut x = room.x - line.x;
    row(ctx, line, x, "in ", faint);
    x += lead;
    for (at, word) in words.iter().enumerate() {
        if at > 0 {
            row(ctx, line, x, " \u{b7} ", faint);
            x += gap;
        }
        let text = match at {
            0 => elide(ctx, word, &muted, (room.w - lead - tail).max(0.0)),
            _ => word.clone(),
        };
        row(ctx, line, x, &text, muted);
        x += ctx.measure(&text, &muted);
    }
    let edge = ctx.styles.line();
    hairline(ctx, line, edge);
    rest
}
