use groove_gfx::{Frame, Rect, Theme, Weight};

use crate::layout::Layout;
use crate::view::{mono, sans};
use crate::widget::{hairline, row};

/// The command palette, on its own layer over everything.
pub fn draw(frame: &mut Frame, theme: &Theme, layout: &Layout) {
    let p = &theme.palette;
    frame.layer();
    frame.quad(frame_rect(layout), p.crust.with_alpha(120));
    let w = layout.px(560.0).min(frame_rect(layout).w - layout.px(32.0));
    let x = (frame_rect(layout).w - w) / 2.0;
    let rect = Rect::new(x, layout.px(96.0), w, layout.row * 2.0 + layout.px(8.0));
    frame.quad(rect, p.mantle);
    frame.border(rect, p.surface1);
    let input = Rect::new(rect.x, rect.y + layout.px(4.0), rect.w, layout.row);
    row(
        frame,
        input,
        layout.px(12.0),
        ">",
        mono(theme, layout, p.text),
    );
    hairline(frame, input, p.surface0);
    let empty = Rect::new(rect.x, input.bottom(), rect.w, layout.row);
    row(
        frame,
        empty,
        layout.px(12.0),
        "no commands yet",
        sans(layout, theme.small, Weight::Regular, p.subtext0),
    );
}

fn frame_rect(layout: &Layout) -> Rect {
    Rect::new(0.0, 0.0, layout.rail.w + layout.header.w, layout.rail.h)
}
