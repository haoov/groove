use groove_gfx::Rect;
use groove_types::{Worktree, WorktreeDelivery};

use crate::components::{delivered, room_for};
use crate::ctx::Ctx;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::after_mark;
use groove_ui_kit::text::{elide, row};
use groove_ui_kit::widgets::{counts, counts_room};

/// One worktree: its branch cut short and its MR beside it, then git at the row's right end.
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
    let right = line.right() - ctx.tokens.md;
    let counted = right - counts_room(ctx, &git);
    counts(ctx, line, counted, &git);
    let mr = room_for(ctx, delivery);
    let gap = if mr > 0.0 { ctx.tokens.sm } else { 0.0 };
    let room = (counted - mr - gap - line.x - indent - ctx.tokens.md).max(0.0);
    let text = elide(ctx, &worktree.branch, &style, room);
    row(ctx, line, indent, &text, style);
    let after = line.x + indent + ctx.measure(&text, &style) + gap;
    delivered(ctx, line, after, delivery);
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
