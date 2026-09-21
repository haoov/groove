//! The sidebar's commits: the branch's own newest first, then the base's.

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::CommitEntry;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::style::Role;
use crate::widget::{elide, row, ruled};

pub(super) fn draw(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui) {
    let log = &app.workspace.log;
    if log.is_empty() {
        let style = ctx.styles.small(Role::Faint);
        let line = Rect::new(body.x, body.y, body.w, ctx.tokens.row);
        return row(ctx, line, ctx.tokens.md, said(app), style);
    }
    let height = ctx.tokens.row;
    let extent = (height * log.len() as f32 - body.h).max(0.0);
    ctx.scrolls(Scroller::Files, extent);
    let scroll = ui.session.files.min(extent);
    let shown = app.workspace.commit.as_ref().map(|one| one.sha.as_str());
    ctx.clipped(body, |ctx| {
        let mut y = body.y - scroll;
        for one in log {
            let line = Rect::new(body.x, y, body.w, height);
            if y + height >= body.y && y <= body.bottom() {
                entry(ctx, line, one, shown == Some(one.sha.as_str()), ui);
            }
            y += height;
        }
    });
}

/// What the list says instead of rows.
fn said(app: &AppState) -> &'static str {
    match app
        .session
        .selected()
        .and_then(|open| open.selected_worktree())
    {
        Some(_) => "reading the commits…",
        None => "no worktree to read",
    }
}

/// One commit: its short name, what it says, and who made it.
fn entry(ctx: &mut Ctx, line: Rect, one: &CommitEntry, shown: bool, ui: &Ui) {
    let target = Target::Commit(one.sha.clone());
    if ui.hover.as_ref() == Some(&target) {
        ctx.quad(line, ctx.styles.hover());
    }
    if shown {
        ruled(ctx, line, ctx.styles.here());
    }
    ctx.hit(line, target);
    let role = match one.is_base {
        true => Role::Ghost,
        false => Role::Muted,
    };
    let name = ctx.styles.code(role);
    let width = ctx.measure(&one.short_sha, &name);
    let at = ctx.tokens.md;
    row(ctx, line, at, &one.short_sha, name);
    let words = match one.is_base {
        true => ctx.styles.body(Role::Faint),
        false => ctx.styles.body(Role::Text),
    };
    let by = author(ctx, line, one);
    let start = at + width + ctx.tokens.sm;
    let room = (by - line.x - start - ctx.tokens.sm).max(0.0);
    let text = elide(
        ctx,
        one.message.lines().next().unwrap_or_default(),
        &words,
        room,
    );
    row(ctx, line, start, &text, words);
}

/// Who made it, at the row's own end. Returns where it starts.
fn author(ctx: &mut Ctx, line: Rect, one: &CommitEntry) -> f32 {
    let style = ctx.styles.small(Role::Ghost);
    let width = ctx.measure(&one.author, &style);
    let at = line.right() - ctx.tokens.md - width;
    row(
        ctx,
        Rect::new(at, line.y, width, line.h),
        0.0,
        &one.author,
        style,
    );
    at
}
