//! The head of the list: what it is showing, and what the change is read against.

use groove_controllers::workspace_service::FOUND_MAX;
use groove_gfx::{Edges, Rect};
use groove_types::{CommitEntry, DiffMode};

use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::hairline;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, Tab, Text};

/// How much the search across the worktree has turned up.
pub(super) fn found(ctx: &mut Ctx, rect: Rect, count: usize) {
    let label = match count {
        0 => "nothing found".to_string(),
        n if n >= FOUND_MAX => format!("found {n}+"),
        n => format!("found · {n}"),
    };
    groove_ui_kit::widgets::Heading::new(&label).draw(ctx, rect);
    hairline(ctx, rect, ctx.styles.line());
}

/// The commit the surface shows, and what puts the working tree back.
pub(super) fn showing(ctx: &mut Ctx, rect: Rect, one: &CommitEntry) {
    hairline(ctx, rect, ctx.styles.line());
    let (raised, action, sm) = (ctx.styles.raised(), ctx.styles.action(), ctx.tokens.sm);
    let working = Button::new("working tree", Target::Working, Role::Muted, raised).hover(action);
    let mut room = rect.pad(Edges::across(ctx.tokens.md, sm));
    working.right(ctx, &mut room, sm);
    Label::new("showing commit", ctx.styles.small(Role::Faint)).left(ctx, &mut room, sm);
    Label::new(&one.short_sha, ctx.styles.code(Role::Text)).draw(ctx, room);
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
