//! The sidebar's commits: the branch's own newest first, then the base's.

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::CommitEntry;

use crate::Ui;
use crate::base::ctx::Ctx;
use crate::base::hit::{Scroller, Target};
use crate::base::style::Role;
use crate::shape::ruled;
use crate::text::Label;
use crate::text::row;
use crate::widgets::scrolled;
use groove_gfx::Edges;

pub(super) fn draw(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui) {
    let log = &app.workspace.log;
    if log.is_empty() {
        let style = ctx.styles.small(Role::Faint);
        let line = Rect::new(body.x, body.y, body.w, ctx.tokens.row);
        return row(ctx, line, ctx.tokens.md, said(app), style);
    }
    let height = ctx.tokens.row;
    let shown = app.workspace.commit.as_ref().map(|one| one.sha.as_str());
    let at = (Scroller::Files, ui.offset(Scroller::Files));
    scrolled(
        ctx,
        body,
        at,
        log,
        |_| height,
        |ctx, line, one| entry(ctx, line, one, shown == Some(one.sha.as_str())),
    );
}

/// What the list says instead of rows.
fn said(app: &AppState) -> &'static str {
    match app.session.selected_worktree() {
        Some(_) => "reading the commits…",
        None => "no worktree to read",
    }
}

/// One commit: its short name, what it says, and who made it.
fn entry(ctx: &mut Ctx, line: Rect, one: &CommitEntry, shown: bool) {
    let target = Target::Commit(one.sha.clone());
    if ctx.hovered(&target) {
        ctx.quad(line, ctx.styles.hover());
    }
    if shown {
        ruled(ctx, line, ctx.styles.here());
    }
    ctx.hit(line, target);
    let (name, words) = match one.is_base {
        true => (ctx.styles.code(Role::Ghost), ctx.styles.body(Role::Faint)),
        false => (ctx.styles.code(Role::Muted), ctx.styles.body(Role::Text)),
    };
    let sm = ctx.tokens.sm;
    let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let by = ctx.styles.small(Role::Ghost);
    Label::new(&one.author, by).right(ctx, &mut room, sm);
    Label::new(&one.short_sha, name).left(ctx, &mut room, sm);
    let said = one.message.lines().next().unwrap_or_default();
    Label::new(said, words).draw(ctx, room);
}
