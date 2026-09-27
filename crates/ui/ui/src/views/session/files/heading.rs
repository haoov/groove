//! The head of the list: what it is showing, and what the change is read against.

use groove_controllers::AppState;
use groove_controllers::workspace_service::FOUND_MAX;
use groove_gfx::{Edges, Rect};
use groove_types::DiffMode;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::views::session::Tab;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::hairline;
use groove_ui_kit::text::{Label, row};

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

/// In the diff what the change is read against; in Files the directory the tree stands in.
pub(super) fn heading(ctx: &mut Ctx, rect: Rect, app: &AppState, ui: &Ui) {
    hairline(ctx, rect, ctx.styles.line());
    if ui.session.tab == Tab::Diff {
        return modes(ctx, rect, app.workspace.mode);
    }
    let dir = app.session.selected_worktree().map(|one| one.dir());
    let name = dir
        .as_ref()
        .and_then(|dir| dir.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let room = rect.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    Label::new(&name, ctx.styles.label(Role::Muted)).draw(ctx, room);
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
