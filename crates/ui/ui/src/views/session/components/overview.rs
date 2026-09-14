use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::Rect;

use super::worktree_row;
use crate::ctx::Ctx;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{Row, list, row};

/// The overview tab: the properties, then the repos with their worktrees, then the body.
pub fn draw(ctx: &mut Ctx, app: &AppState, area: Rect) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let y = section(ctx, area, area.y, "Repos and worktrees");
    repos(ctx, open, area, y);
}

/// A section's name, and the y its content starts at.
fn section(ctx: &mut Ctx, area: Rect, y: f32, title: &str) -> f32 {
    let style = ctx.styles.heading(Role::Faint);
    let pad = ctx.tokens.md;
    let line = Rect::new(area.x, y, area.w, ctx.tokens.row);
    row(ctx, line, pad, &title.to_uppercase(), style);
    line.bottom()
}

/// One block per repo: the repo, then its worktrees.
fn repos(ctx: &mut Ctx, open: &Open, area: Rect, top: f32) {
    let pad = ctx.tokens.md;
    if open.repos.is_empty() {
        let style = ctx.styles.body(Role::Faint);
        let line = Rect::new(area.x, top, area.w, ctx.tokens.row);
        return row(ctx, line, pad, "No repos. Add one from the palette.", style);
    }
    let (name, slug) = (ctx.styles.label(Role::Text), ctx.styles.small(Role::Faint));
    let at_slug = ctx.tokens.aside_mid;
    let mut y = top;
    for repo in &open.repos {
        let head = Row::new(pad, &repo.project, name).mark(Mark::Repo).aside(
            at_slug,
            repo.id.as_str(),
            slug,
        );
        y = list(
            ctx,
            Rect::new(area.x, y, area.w, ctx.tokens.row),
            &[head],
            None,
        );
        for worktree in open.worktrees.iter().filter(|w| w.repo == repo.id) {
            let line = Rect::new(area.x, y, area.w, ctx.tokens.row);
            worktree_row::draw(ctx, line, worktree, open.delivery_of(&worktree.id));
            y += ctx.tokens.row;
        }
        y += ctx.tokens.sm;
    }
}
