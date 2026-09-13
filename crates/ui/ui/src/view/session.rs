use groove_controllers::AppState;
use groove_gfx::{Frame, Rect, Theme, Weight};

use crate::layout::Layout;
use crate::view::{mono, sans};
use crate::widget::{hairline, row};

/// The session surface: one header line, then the agent pane, the workspace and the sidebar.
pub fn draw(frame: &mut Frame, app: &AppState, theme: &Theme, layout: &Layout) {
    let p = &theme.palette;
    frame.quad(layout.header, p.mantle);
    hairline(frame, layout.header, p.surface0);

    match app.session.selected() {
        Some(open) => {
            row(
                frame,
                layout.header,
                layout.px(12.0),
                &open.session.title,
                sans(layout, theme.title, Weight::SemiBold, p.text),
            );
            if let Some(wt) = open.worktrees.first() {
                let branch = Rect::new(
                    layout.header.x + layout.px(320.0),
                    layout.header.y,
                    layout.header.w - layout.px(320.0),
                    layout.header.h,
                );
                row(
                    frame,
                    branch,
                    0.0,
                    &wt.branch,
                    mono(theme, layout, p.lavender),
                );
            }
        }
        None => {
            row(
                frame,
                layout.header,
                layout.px(12.0),
                "Groove",
                sans(layout, theme.title, Weight::SemiBold, p.text),
            );
            let hint = Rect::new(
                layout.body.x,
                layout.body.y + layout.px(8.0),
                layout.body.w,
                layout.row,
            );
            row(
                frame,
                hint,
                layout.px(12.0),
                "No session open. Open one from the board.",
                sans(layout, theme.text, Weight::Regular, p.subtext0),
            );
        }
    }
}
