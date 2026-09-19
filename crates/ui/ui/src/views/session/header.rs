use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::hit::{Picks, Target};
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{after_mark, divider, hairline, icon, leading, picker, row};

/// The workspace's first line: what the session is, then what every tab follows.
pub fn draw(ctx: &mut Ctx, app: &AppState) {
    let rect = ctx.layout.header;
    let (panel, line) = (ctx.styles.panel(), ctx.styles.line());
    ctx.quad(rect, panel);
    hairline(ctx, rect, line);

    let pad = ctx.tokens.md;
    let Some(open) = app.session.selected() else {
        let title = ctx.styles.title(Role::Text);
        return row(ctx, rect, pad, "Groove", title);
    };

    let mark = Mark::of_kind(&open.session.kind);
    let box_ = leading(ctx, rect, rect.x + pad);
    icon(ctx, box_, mark, Role::Faint);

    let title = ctx.styles.title(Role::Text);
    let mut x = rect.x + after_mark(ctx, pad);
    let width = ctx.measure(&open.session.title, &title);
    row(
        ctx,
        Rect::new(x, rect.y, width, rect.h),
        0.0,
        &open.session.title,
        title,
    );
    x += width + ctx.tokens.md;

    x = divider(ctx, rect, x) + ctx.tokens.md;
    pickers(ctx, rect, x, open);
}

/// The repo and the worktree the session points at; every tab follows them.
fn pickers(ctx: &mut Ctx, line: Rect, x: f32, open: &Open) {
    let worktree = open.selected_worktree();
    let repo = worktree
        .and_then(|w| open.repos.iter().find(|r| r.id == w.repo))
        .map(|r| r.project.as_str());
    let (repo_role, branch_role) = (role_of(repo), role_of(worktree.map(|w| w.branch.as_str())));
    let repo_box = picker(ctx, line, x, repo.unwrap_or("no repo"), repo_role);
    ctx.hit(repo_box, Target::Picker(Picks::Repo));
    let x = separator(ctx, line, repo_box.right() + ctx.tokens.sm);
    let branch = worktree.map(|w| w.branch.as_str()).unwrap_or("no worktree");
    let branch_box = picker(ctx, line, x, branch, branch_role);
    ctx.hit(branch_box, Target::Picker(Picks::Branch));
}

fn role_of(value: Option<&str>) -> Role {
    match value {
        Some(_) => Role::Muted,
        None => Role::Ghost,
    }
}

/// The dot between two pickers.
fn separator(ctx: &mut Ctx, line: Rect, x: f32) -> f32 {
    let style = ctx.styles.body(Role::Ghost);
    let width = ctx.measure("·", &style);
    row(ctx, Rect::new(x, line.y, width, line.h), 0.0, "·", style);
    x + width + ctx.tokens.sm
}
