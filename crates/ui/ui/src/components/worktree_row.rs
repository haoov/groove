use groove_gfx::Rect;
use groove_types::{Worktree, WorktreeDelivery};

use crate::components::{delivered, room_for};
use crate::ctx::Ctx;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, row_in};
use groove_ui_kit::shape::after_mark;
use groove_ui_kit::text::{elide, row};
use groove_ui_kit::widgets::{counts, counts_room};

/// One worktree in a line from the left: its branch, what git counts, then its MR.
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
    let (md, sm, wide) = (ctx.tokens.md, ctx.tokens.sm, counts_room(ctx, &git));
    let mr = room_for(ctx, delivery);
    let fixed = |width: f32| Spec::default().width(width);
    let [_, room, _] = row_in(
        line,
        [fixed(indent), Spec::fill(), fixed(md + wide + sm + mr)],
    );
    let text = elide(ctx, &worktree.branch, &style, room.w);
    row(ctx, line, indent, &text, style);
    let drawn = fixed(ctx.measure(&text, &style));
    let [.., after] = row_in(line, [fixed(indent), drawn, fixed(md), Spec::fill()]);
    let x = counts(ctx, line, after.x, &git);
    let gap = if wide > 0.0 { sm } else { 0.0 };
    delivered(ctx, line, x + gap, delivery);
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
