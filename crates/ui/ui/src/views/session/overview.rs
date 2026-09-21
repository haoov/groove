mod mr;
mod task;

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::Rect;

use super::worktree_row;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{Row, list, row};

/// The overview tab, scrolled: the properties, the repos with their worktrees, the body.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, area: Rect) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let top = area.y - ui.session.overview;
    let mut bottom = top;
    ctx.clipped(area, |ctx| {
        let mut y = properties(ctx, app, open, area, top, ui);
        y = section(ctx, area, y, "Repos and worktrees", y > top);
        y = repos(ctx, open, area, y);
        y = merge_request(ctx, app, area, y);
        bottom = body(ctx, app, open, area, y);
    });
    let height = bottom - top + ctx.tokens.md;
    ctx.scrolls(Scroller::Overview, (height - area.h).max(0.0));
}

/// The task's six properties, for a session that works one.
fn properties(ctx: &mut Ctx, app: &AppState, open: &Open, area: Rect, top: f32, ui: &Ui) -> f32 {
    let Some(one) = app.task.worked(&open.session) else {
        return top;
    };
    let y = section(ctx, area, top, "Properties", false);
    let time = app.task.measured(&one.external_id);
    task::properties(ctx, area, y, one, time, ui) + ctx.tokens.sm
}

/// The selected worktree's MR, when its forge has answered for it.
fn merge_request(ctx: &mut Ctx, app: &AppState, area: Rect, top: f32) -> f32 {
    if app.workspace.delivery.read.is_none() {
        return top;
    }
    let y = section(ctx, area, top, "Merge request", true);
    mr::rows(ctx, area, y, &app.workspace.delivery)
}

/// The task's body, under everything the session holds.
fn body(ctx: &mut Ctx, app: &AppState, open: &Open, area: Rect, top: f32) -> f32 {
    let Some(one) = app.task.worked(&open.session) else {
        return top;
    };
    let Some(text) = app.task.body(&one.short_id).filter(|text| !text.is_empty()) else {
        return top;
    };
    let y = section(ctx, area, top, "Body", true);
    task::body(ctx, area, y, text)
}

/// Whether a scrolled line is inside the tab.
fn seen(area: Rect, line: Rect) -> bool {
    line.bottom() > area.y && line.y < area.bottom()
}

/// A section's name under a rule when one stands above it. Returns where its content starts.
fn section(ctx: &mut Ctx, area: Rect, y: f32, title: &str, under: bool) -> f32 {
    let pad = ctx.tokens.md;
    let mut line = Rect::new(area.x, y, area.w, ctx.tokens.row);
    if under {
        line = Rect::new(area.x, y + ctx.tokens.sm, area.w, ctx.tokens.row);
        let rule = Rect::new(area.x + pad, y, area.w - pad * 2.0, ctx.tokens.hairline);
        ctx.quad(rule, ctx.styles.line());
    }
    let style = ctx.styles.heading(Role::Faint);
    row(ctx, line, pad, &title.to_uppercase(), style);
    line.bottom()
}

/// One block per repo: the repo, then its worktrees. Returns the y under the last.
fn repos(ctx: &mut Ctx, open: &Open, area: Rect, top: f32) -> f32 {
    let pad = ctx.tokens.md;
    if open.repos.is_empty() {
        let style = ctx.styles.body(Role::Faint);
        let line = Rect::new(area.x, top, area.w, ctx.tokens.row);
        row(ctx, line, pad, "No repos. Add one from the palette.", style);
        return line.bottom();
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
            if seen(area, line) {
                ctx.hit(line, Target::Worktree(worktree.id.clone()));
            }
            y += ctx.tokens.row;
        }
        y += ctx.tokens.sm;
    }
    y
}
