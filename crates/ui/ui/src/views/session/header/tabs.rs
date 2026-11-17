//! The workspace's tabs at the header's right, one group of segments on the header's band.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::views::session::Tab;

/// The tabs right to left from `line`'s end, the open one held. Returns the room left of them.
pub(super) fn draw(ctx: &mut Ctx, line: Rect, app: &AppState, ui: &crate::Ui) -> Rect {
    let xs = ctx.tokens.xs;
    let mut room = line.pad(Edges::across(0.0, xs));
    let side = groove_ui_kit::widgets::Side::Right;
    groove_ui_kit::widgets::tab_group(ctx, &mut room, shown(app, ui), (side, 0.0));
    room
}

/// How wide the tabs stand, with their margin: what `draw` takes from the line.
pub(super) fn width(ctx: &mut Ctx, app: &AppState, ui: &crate::Ui) -> f32 {
    let tabs = shown(app, ui);
    ctx.tokens.xs + groove_ui_kit::widgets::group_width(ctx, &tabs)
}

/// The tabs the session has, the open one held; Resources only while it holds a cluster.
fn shown(app: &AppState, ui: &crate::Ui) -> Vec<groove_ui_kit::widgets::Tab<'static, Target>> {
    let held = app
        .session
        .selected()
        .is_some_and(|open| open.holds_clusters());
    let up = Tab::ALL
        .into_iter()
        .filter(|tab| *tab != Tab::Resources || held);
    up.map(|tab| {
        let on = tab == ui.session.tab && !ui.session.stowed;
        groove_ui_kit::widgets::Tab::new(tab.label(), Target::Tab(tab), on)
    })
    .collect()
}
