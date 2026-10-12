use groove_gfx::{Align, Edges, Rect};
use groove_types::{Worktree, WorktreeDelivery};

use crate::components::{delivered, room_for};
use crate::ctx::Ctx;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, row_in};
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
    let (md, wide) = (ctx.tokens.md, counts_room(ctx, &git));
    let end = line.pad(Edges::across(0.0, md));
    let counted = end.align((wide, line.h), Align::End, Align::Start);
    counts(ctx, line, counted.x, &git);
    let mr = room_for(ctx, delivery);
    let gap = if mr > 0.0 { ctx.tokens.sm } else { 0.0 };
    let fixed = |width: f32| Spec::default().width(width);
    let kept = fixed(md + mr + gap + md + wide);
    let [_, room, _] = row_in(line, [fixed(indent), Spec::fill(), kept]);
    let text = elide(ctx, &worktree.branch, &style, room.w);
    row(ctx, line, indent, &text, style);
    let drawn = fixed(ctx.measure(&text, &style));
    let [.., after] = row_in(line, [fixed(indent), drawn, fixed(gap), Spec::fill()]);
    delivered(ctx, line, after.x, delivery);
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
