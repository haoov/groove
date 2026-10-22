//! The workspace's tabs at the header's right, each a rounded row on the glass.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::views::session::Tab;

/// The tabs right to left from `line`'s end, the open one held. Returns the room left of them.
pub(super) fn draw(ctx: &mut Ctx, line: Rect, app: &AppState, ui: &crate::Ui) -> Rect {
    let held = app
        .session
        .selected()
        .is_some_and(|open| open.holds_clusters());
    let xs = ctx.tokens.xs;
    let mut room = line.pad(Edges::across(0.0, xs));
    let up: Vec<Tab> = Tab::ALL
        .into_iter()
        .filter(|tab| *tab != Tab::Resources || held)
        .collect();
    for tab in up.into_iter().rev() {
        let on = tab == ui.session.tab && !ui.session.stowed;
        let shown = groove_ui_kit::widgets::Tab::new(tab.label(), Target::Tab(tab), on).glass();
        shown.right(ctx, &mut room, xs);
    }
    room
}
