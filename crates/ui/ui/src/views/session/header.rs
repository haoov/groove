//! The session's title and the worktree it follows, on its bar.

use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};

use crate::ctx::Ctx;
use crate::hit::{Picks, Target};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, row_in};
use groove_ui_kit::text::{elide, row};
use groove_ui_kit::widgets::{Button, lead};

/// The session's kind and its title, then what opens its page; returns the room left after it.
pub(super) fn titled_rest(ctx: &mut Ctx, line: Rect, open: &Open, page: Option<&str>) -> Rect {
    let (md, size) = (ctx.tokens.md, ctx.tokens.icon);
    let mut room = line.pad(Edges::across(md, md));
    lead(
        ctx,
        &mut room,
        Mark::of_kind(&open.session.kind),
        Role::Faint,
    );
    let style = ctx.styles.session_title(Role::Text);
    let wide = match page {
        Some(_) => size + ctx.tokens.sm * 2.0 + ctx.tokens.sm,
        None => 0.0,
    };
    let title = elide(ctx, &open.session.title, &style, (room.w - wide).max(0.0));
    let fixed = |width: f32| Spec::default().width(width);
    let drawn = fixed(ctx.measure(&title, &style));
    let [_, _, after] = row_in(room, [drawn, fixed(ctx.tokens.sm), Spec::fill()]);
    row(ctx, room, 0.0, &title, style);
    let Some(url) = page else {
        return after;
    };
    let target = Target::TaskPage(url.to_string());
    let outward =
        Button::icon(Mark::Outward, 0, target, Role::Faint).hover(ctx.styles.glass_hover());
    let drawn = outward.at(ctx, line, after.x);
    let [_, rest] = row_in(after, [fixed(drawn.w + ctx.tokens.sm), Spec::fill()]);
    rest
}

/// How wide `groove · feat/x` stands whole, its padding counted.
pub(super) fn worktree_wide(ctx: &mut Ctx, open: &Open) -> f32 {
    let (repo, branch) = named(open);
    let said = format!("{repo} · {branch}");
    ctx.measure(&said, &ctx.styles.code(Role::Faint)) + ctx.tokens.sm * 2.0
}

/// `groove · feat/x`, the worktree every tab follows; a click picks another.
pub(super) fn worktree_of(ctx: &mut Ctx, room: Rect, open: &Open) {
    let (repo, branch) = named(open);
    let said = format!("{repo} · {branch}");
    let style = ctx.styles.code(Role::Faint);
    let pad = ctx.tokens.sm;
    let text = elide(ctx, &said, &style, (room.w - pad * 2.0).max(0.0));
    let fixed = |width: f32| Spec::default().width(width);
    let wide = ctx.measure(&text, &style) + pad * 2.0;
    let [slot, _] = row_in(room, [fixed(wide), Spec::fill()]);
    groove_ui_kit::widgets::glass_hoverable(ctx, slot, Target::Picker(Picks::Branch));
    row(ctx, slot, pad, &text, style);
}

/// The repo and the branch the line names, each as it stands or as what it lacks.
fn named(open: &Open) -> (&str, &str) {
    let repo = open.selected_repo().map(|r| r.project.as_str());
    let branch = open.selected_worktree().map(|w| w.branch.as_str());
    (repo.unwrap_or("no repo"), branch.unwrap_or("no worktree"))
}
