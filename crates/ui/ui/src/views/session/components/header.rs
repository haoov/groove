use groove_controllers::AppState;
use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{hairline, icon, leading, row};

/// One line across the whole session: the title, then the selected worktree.
pub fn draw(ctx: &mut Ctx, app: &AppState) {
    let rect = ctx.layout.header;
    let (panel, line) = (ctx.styles.panel(), ctx.styles.line());
    ctx.quad(rect, panel);
    hairline(ctx, rect, line);

    let pad = ctx.tokens.md;
    let title = ctx.styles.title(Role::Text);
    let Some(open) = app.session.selected() else {
        return row(ctx, rect, pad, "Groove", title);
    };
    let mark = Mark::of_kind(&open.session.kind);
    let box_ = leading(ctx, rect, rect.x + pad);
    icon(ctx, box_, mark, Role::Faint);
    let indent = pad + ctx.tokens.icon + ctx.tokens.sm;
    row(ctx, rect, indent, &open.session.title, title);

    let Some(worktree) = open.selected_worktree() else {
        return;
    };
    let branch = ctx.styles.body(Role::Accent);
    let at = ctx.tokens.aside_far;
    let area = Rect::new(rect.x + at, rect.y, rect.w - at, rect.h);
    row(ctx, area, 0.0, &worktree.branch, branch);
}
