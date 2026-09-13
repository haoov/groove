use groove_gfx::{Frame, Rect, Theme, Weight};

use groove_controllers::AppState;

use crate::layout::Layout;
use crate::palette::Palette;
use crate::view::{mono, sans};
use crate::widget::{hairline, row};

const MAX_ROWS: usize = 8;

/// The command palette, on its own layer over everything.
pub fn draw(frame: &mut Frame, app: &AppState, palette: &Palette, theme: &Theme, layout: &Layout) {
    let p = &theme.palette;
    let window = Rect::new(0.0, 0.0, layout.rail.w + layout.header.w, layout.rail.h);
    let rows = palette.rows(app);
    let shown = rows.len().clamp(1, MAX_ROWS);

    frame.layer();
    frame.quad(window, p.crust.with_alpha(120));
    let w = layout.px(560.0).min(window.w - layout.px(32.0));
    let x = (window.w - w) / 2.0;
    let rect = Rect::new(
        x,
        layout.px(96.0),
        w,
        layout.row * (shown as f32 + 1.0) + layout.px(8.0),
    );
    frame.quad(rect, p.mantle);
    frame.border(rect, p.surface1);

    let input = Rect::new(rect.x, rect.y + layout.px(4.0), rect.w, layout.row);
    let text = format!("> {}▏", palette.query);
    row(
        frame,
        input,
        layout.px(12.0),
        &text,
        mono(theme, layout, p.text),
    );
    hairline(frame, input, p.surface0);

    if rows.is_empty() {
        let empty = Rect::new(rect.x, input.bottom(), rect.w, layout.row);
        row(
            frame,
            empty,
            layout.px(12.0),
            "no match",
            sans(layout, theme.small, Weight::Regular, p.subtext0),
        );
        return;
    }
    let first = palette.selected.saturating_sub(MAX_ROWS - 1);
    for (i, entry) in rows.iter().enumerate().skip(first).take(MAX_ROWS) {
        let y = input.bottom() + layout.row * (i - first) as f32;
        let line = Rect::new(rect.x + 1.0, y, rect.w - 2.0, layout.row);
        if i == palette.selected {
            frame.quad(line, p.surface0);
        }
        row(
            frame,
            line,
            layout.px(12.0),
            entry.group,
            sans(layout, theme.small, Weight::Regular, p.subtext0),
        );
        let label = Rect::new(
            line.x + layout.px(90.0),
            y,
            line.w - layout.px(90.0),
            layout.row,
        );
        row(
            frame,
            label,
            0.0,
            &entry.label,
            sans(layout, theme.text, Weight::Medium, p.text),
        );
    }
}
