//! The line under the region that holds the keyboard.

use groove_controllers::AppState;
use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::{Focus, Surface, Ui};

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let Some(region) = region(ctx, app, ui).filter(|one| !one.is_empty()) else {
        return;
    };
    groove_ui_kit::shape::focus_line(ctx, region);
}

/// The focused region's box, whose bottom edge takes the line; none off the session.
fn region(ctx: &Ctx, app: &AppState, ui: &Ui) -> Option<Rect> {
    let layout = &ctx.app.layout;
    if ui.focus == Focus::Rail {
        return Some(layout.rail);
    }
    if ui.showing(app) == Surface::Board {
        let list = ui.board.list?;
        return Some(crate::views::board::column_of(
            &ctx.tokens,
            layout.board,
            list,
        ));
    }
    app.session.selected()?;
    let lowest = |a: Rect, b: Rect| if b.is_empty() { a } else { b };
    match ui.focus {
        Focus::Rail => None,
        Focus::Agent => Some(lowest(layout.agent, layout.agent_bar)),
        Focus::Workspace => Some(layout.workspace),
        Focus::Sidebar => {
            (!layout.sidebar.is_empty()).then(|| lowest(layout.sidebar, layout.commit))
        }
        Focus::Terminal => Some(layout.manual),
    }
}
