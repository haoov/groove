use groove_controllers::AppState;
use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::style::Role;
use crate::widget::{Row, list, row, tabs};

/// The overview tab: the repos, each with its worktrees as rows.
pub fn draw(ctx: &mut Ctx, app: &AppState) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let area = ctx.layout.workspace;
    let strip = Rect::new(area.x, area.y, area.w, ctx.tokens.row);
    tabs(ctx, strip, &["overview"], 0);

    let body = Rect::new(
        area.x,
        strip.bottom() + ctx.tokens.sm,
        area.w,
        area.h - ctx.tokens.row,
    );
    if open.repos.is_empty() {
        let style = ctx.styles.body(Role::Muted);
        let pad = ctx.tokens.md;
        let rect = Rect::new(body.x, body.y, body.w, ctx.tokens.row);
        return row(ctx, rect, pad, "No repos. Add one from the palette.", style);
    }

    let (name, slug) = (ctx.styles.strong(Role::Text), ctx.styles.small(Role::Muted));
    let (branch, base) = (ctx.styles.mono(Role::Text), ctx.styles.mono(Role::Muted));
    let (pad, indent) = (ctx.tokens.md, ctx.tokens.indent);
    let (at_slug, at_base) = (ctx.tokens.aside_mid, ctx.tokens.aside_far);
    let mut y = body.y;
    for repo in &open.repos {
        let worktrees: Vec<_> = open
            .worktrees
            .iter()
            .filter(|w| w.repo == repo.id)
            .collect();
        let bases: Vec<String> = worktrees
            .iter()
            .map(|w| {
                w.base_ref
                    .as_deref()
                    .map(|b| format!("→ {b}"))
                    .unwrap_or_default()
            })
            .collect();
        let mut rows =
            vec![Row::new(pad, &repo.project, name).aside(at_slug, repo.id.as_str(), slug)];
        let mut selected = None;
        for (i, worktree) in worktrees.iter().enumerate() {
            if open.state.selected_worktree.as_ref() == Some(&worktree.id) {
                selected = Some(i + 1);
            }
            rows.push(Row::new(indent, &worktree.branch, branch).aside(at_base, &bases[i], base));
        }
        y = list(ctx, Rect::new(body.x, y, body.w, body.h), &rows, selected) + ctx.tokens.sm;
    }
}
