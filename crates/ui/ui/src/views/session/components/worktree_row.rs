use groove_gfx::Rect;
use groove_types::{Worktree, WorktreeDelivery};

use crate::ctx::Ctx;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{after_mark, counts, row};

/// One worktree: its branch, then what git says about it as icons and counts.
/// Its text starts under the repo's name, and the counts follow the branch.
pub fn draw(ctx: &mut Ctx, line: Rect, worktree: &Worktree, delivery: Option<&WorktreeDelivery>) {
    let branch = ctx.styles.body(Role::Muted);
    let indent = after_mark(ctx, ctx.tokens.md);
    row(ctx, line, indent, &worktree.branch, branch);

    let Some(delivery) = delivery else {
        return;
    };
    let status = delivery.status;
    let width = ctx.measure(&worktree.branch, &branch);
    let at = line.x + indent + width + ctx.tokens.lg;
    counts(
        ctx,
        line,
        at,
        &[
            (Mark::Ahead, status.ahead, Role::Working),
            (Mark::Behind, status.behind, Role::Ghost),
            (Mark::Staged, status.staged, Role::Ok),
            (Mark::Modified, status.modified, Role::Warn),
        ],
    );
}
