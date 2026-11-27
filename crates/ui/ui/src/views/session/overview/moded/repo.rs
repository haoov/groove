//! One repo apart on its card: its name and remote, then a row each worktree while open.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::{CiState, Repo, Worktree};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{from_top, top_rule};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::flat_row;

use crate::components::worktree_row;
use crate::ctx::Ctx;
use crate::hit::Target;

/// The card: the repo's head, then a row each worktree while open.
pub(super) fn draw(
    ctx: &mut Ctx,
    app: &AppState,
    open: &Open,
    rect: Rect,
    (repo, held, shown): (&Repo, &[&Worktree], bool),
) {
    let mut room = super::carded(ctx, rect);
    let mut top = super::headed(ctx, &mut room, (repo.id.as_str(), shown));
    Label::new(&repo.project, ctx.styles.label(Role::Text)).left(ctx, &mut top, ctx.tokens.md);
    Label::new(repo.id.as_str(), ctx.styles.small(Role::Faint)).draw(ctx, top);
    if !shown {
        return;
    }
    if held.is_empty() {
        let line = from_top(&mut room, ctx.tokens.row, 0.0);
        let line = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        Label::new("No worktree.", ctx.styles.body(Role::Faint)).draw(ctx, line);
    }
    for one in held {
        let delivery = app.delivery.row(&one.id, open.status_of(&one.id));
        let line = from_top(&mut room, ctx.tokens.row, 0.0);
        top_rule(ctx, line, ctx.styles.panel_edge());
        flat_row(ctx, line, Target::Worktree(one.id.clone()), false);
        worktree_row::draw(ctx, line, one, Some(&delivery));
    }
}

/// A pipeline's colour: green passed, red failed, blue while it runs, faint else.
pub(crate) fn ci_role(state: CiState) -> Role {
    match state {
        CiState::Success => Role::Ok,
        CiState::Failed => Role::Bad,
        CiState::Pending | CiState::Running => Role::Working,
        _ => Role::Faint,
    }
}
