//! The head of the list: what it is showing, and what the change is read against.

use groove_controllers::workspace_service::FOUND_MAX;
use groove_gfx::Rect;
use groove_types::DiffMode;

use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::hairline;
use groove_ui_kit::text::row;
use groove_ui_kit::widgets::{Tab, Text};

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

/// What the change is read against, the one in use raised, at the row's own end.
pub(super) fn modes(ctx: &mut Ctx, line: Rect, current: DiffMode) {
    hairline(ctx, line, ctx.styles.line());
    let mut room = line;
    for mode in DiffMode::ALL.into_iter().rev() {
        let tab = Tab::new(mode.label(), Target::Mode(mode), mode == current);
        let tab = tab.text(Text::Small).quiet(Role::Faint).tight();
        tab.right(ctx, &mut room, 0.0);
    }
}
