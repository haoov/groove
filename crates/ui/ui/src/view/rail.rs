use groove_controllers::AppState;
use groove_gfx::{Color, Frame, Rect, Theme, Weight};
use groove_types::{AgentStatus, AttentionClass};

use crate::layout::Layout;
use crate::view::sans;
use crate::widget::{hairline, row};

/// Opened sessions only, in the order opened; the Board row above, the feed below.
pub fn draw(frame: &mut Frame, app: &AppState, theme: &Theme, layout: &Layout) {
    let p = &theme.palette;
    frame.quad(layout.rail, p.mantle);
    frame.quad(
        Rect::new(layout.rail.right() - 1.0, 0.0, 1.0, layout.rail.h),
        p.surface0,
    );

    let mut y = layout.px(8.0);
    let board = Rect::new(0.0, y, layout.rail.w - 1.0, layout.row);
    row(
        frame,
        board,
        layout.px(12.0),
        "Board",
        sans(layout, theme.text, Weight::Medium, p.text),
    );
    y += layout.row + layout.px(8.0);

    for open in &app.session.open {
        let rect = Rect::new(0.0, y, layout.rail.w - 1.0, layout.row);
        row(
            frame,
            rect,
            layout.px(12.0),
            &open.session.title,
            sans(layout, theme.text, Weight::Medium, p.text),
        );
        let (label, color) = status_of(app, open, theme);
        let status = Rect::new(0.0, y + layout.row, layout.rail.w - 1.0, layout.row);
        row(
            frame,
            status,
            layout.px(12.0),
            label,
            sans(layout, theme.small, Weight::Regular, color),
        );
        y += 2.0 * layout.row;
    }

    let footer = Rect::new(
        0.0,
        layout.rail.h - layout.row,
        layout.rail.w - 1.0,
        layout.row,
    );
    hairline(
        frame,
        Rect::new(0.0, footer.y - layout.row, footer.w, layout.row),
        p.surface0,
    );
    row(
        frame,
        footer,
        layout.px(12.0),
        "settings",
        sans(layout, theme.small, Weight::Regular, p.subtext0),
    );
}

/// The row's second line and its colour: state is colour, nothing else is.
fn status_of(
    app: &AppState,
    open: &groove_controllers::session_service::Open,
    theme: &Theme,
) -> (&'static str, Color) {
    let p = &theme.palette;
    let Some(activity) = app.agent.activity(&open.session.id) else {
        return ("idle", p.overlay0);
    };
    let label = match &activity.status {
        _ if !activity.asks.is_empty() => "asks",
        AgentStatus::Working => "working",
        AgentStatus::Done { .. } => "done",
        AgentStatus::Idle => "idle",
        AgentStatus::Exited { .. } => "exited",
        AgentStatus::Error { .. } => "error",
    };
    let color = match activity.class() {
        AttentionClass::NeedsYou => p.peach,
        AttentionClass::Moving => p.blue,
        AttentionClass::ActWhenYouLook => p.text,
        AttentionClass::Quiet => p.overlay0,
    };
    (label, color)
}
