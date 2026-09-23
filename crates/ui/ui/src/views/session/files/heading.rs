//! The head of the list: what it is showing, and what the change is read against.

use groove_controllers::AppState;
use groove_controllers::workspace_service::FOUND_MAX;
use groove_gfx::Rect;
use groove_types::DiffMode;

use super::super::files::{ROWS_MAX, Scope};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::style::Role;
use crate::widget::{hairline, row, tabs};

/// How much the search across the worktree has turned up.
pub(super) fn found(ctx: &mut Ctx, rect: Rect, count: usize) {
    let style = ctx.styles.heading(Role::Faint);
    let label = match count {
        0 => "NOTHING FOUND".to_string(),
        n if n >= FOUND_MAX => format!("FOUND {n}+"),
        n => format!("FOUND · {n}"),
    };
    row(ctx, rect, ctx.tokens.md, &label, style);
    hairline(ctx, rect, ctx.styles.line());
}

/// The two scopes, the one in use lit and counted, each a tab to click.
pub(super) fn heading(ctx: &mut Ctx, rect: Rect, count: usize, app: &AppState, ui: &Ui) {
    let labels: Vec<String> = Scope::ALL
        .iter()
        .map(|scope| scoped(*scope, count, ui.session.scope == *scope))
        .collect();
    let shown: Vec<&str> = labels.iter().map(String::as_str).collect();
    let at = Scope::ALL
        .iter()
        .position(|scope| *scope == ui.session.scope)
        .unwrap_or(0);
    for (scope, line) in Scope::ALL.iter().zip(tabs(ctx, rect, &shown, at)) {
        ctx.hit(line, Target::Scope(*scope));
    }
    modes(ctx, rect, app.workspace.mode);
}

/// What the change is read against, the one in use raised, at the row's own end.
fn modes(ctx: &mut Ctx, line: Rect, current: DiffMode) {
    let (pad, gap) = (ctx.tokens.sm, ctx.tokens.xs);
    let mut at = line.right();
    for mode in DiffMode::ALL.into_iter().rev() {
        let role = match mode == current {
            true => Role::Text,
            false => Role::Faint,
        };
        let style = ctx.styles.small(role);
        let width = ctx.measure(mode.label(), &style) + pad * 2.0;
        at -= width;
        let box_ = Rect::new(at, line.y, width, line.h);
        if mode == current {
            let raised = ctx.styles.raised();
            ctx.quad(box_, raised);
        }
        row(ctx, box_, pad, mode.label(), style);
        ctx.hit(box_, Target::Mode(mode));
        at -= gap;
    }
}

/// A scope's own name, with what the list holds under it.
fn scoped(scope: Scope, count: usize, here: bool) -> String {
    match (here, count) {
        (true, n) if n >= ROWS_MAX => format!("{} · {n}+", scope.label()),
        (true, n) if n > 0 => format!("{} · {n}", scope.label()),
        _ => scope.label().to_string(),
    }
}
