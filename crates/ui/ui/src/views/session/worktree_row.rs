use groove_gfx::Rect;
use groove_types::{Worktree, WorktreeDelivery};

use crate::base::ctx::Ctx;
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::after_mark;
use crate::text::{elide, row};
use crate::widgets::{counts, counts_room, delivered, room_for};

/// One worktree: its branch cut short, then git and the forge at the row's right end.
pub fn draw(ctx: &mut Ctx, line: Rect, worktree: &Worktree, delivery: Option<&WorktreeDelivery>) {
    let style = ctx.styles.body(Role::Muted);
    let indent = after_mark(ctx, ctx.tokens.md);
    let Some(delivery) = delivery else {
        let text = elide(
            ctx,
            &worktree.branch,
            &style,
            line.w - indent - ctx.tokens.md,
        );
        return row(ctx, line, indent, &text, style);
    };
    let git = told(delivery);
    let needed = counts_room(ctx, &git) + room_for(ctx, delivery);
    let right = line.right() - ctx.tokens.md;
    let room = (right - needed - line.x - indent - ctx.tokens.md).max(0.0);
    let text = elide(ctx, &worktree.branch, &style, room);
    row(ctx, line, indent, &text, style);
    let at = counts(ctx, line, right - needed, &git);
    delivered(ctx, line, at, delivery);
}

/// What git says about the worktree, as the counts it draws.
fn told(delivery: &WorktreeDelivery) -> [(Mark, u32, Role); 4] {
    let status = delivery.status;
    [
        (Mark::Ahead, status.ahead, Role::Working),
        (Mark::Behind, status.behind, Role::Ghost),
        (Mark::Staged, status.staged, Role::Ok),
        (Mark::Modified, status.modified, Role::Warn),
    ]
}
