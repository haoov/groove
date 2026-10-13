//! A fresh frame for a kit test to draw on.

use groove_gfx::{CellSize, Color, Fonts, Frame, Rect, Size};
use groove_types::{ThemeName, Timestamp};

use crate::base::ctx::{App, Ctx, Metrics};
use crate::base::tokens::Tokens;

pub(crate) struct Nothing;

impl App for Nothing {
    type Target = ();

    fn hit(&mut self, _: Rect, _: ()) {}
}

/// Runs `draw` on a fresh frame; returns what it returned and the frame.
pub(crate) fn drawn<R>(draw: impl FnOnce(&mut Ctx<'_, Nothing>) -> R) -> (R, Frame) {
    drawn_at(1.0, draw)
}

/// `drawn` at a window scale.
pub(crate) fn drawn_at<R>(scale: f32, draw: impl FnOnce(&mut Ctx<'_, Nothing>) -> R) -> (R, Frame) {
    let design = Tokens::new(scale);
    let metrics = Metrics {
        size: Size::new(1280, 800),
        scale,
        text: design.text,
        code: design.code,
        terminal: design.code,
        cell: CellSize {
            width: 8.0,
            height: 16.0,
        },
        advance: 8.0,
        tick: 0,
        now: Timestamp::new(0),
    };
    let mut frame = Frame::new(metrics.size, Color::BLACK);
    let mut fonts = Fonts::embedded();
    let theme = ThemeName::default();
    let out = {
        let mut ctx = Ctx::new(theme, metrics, Nothing, &mut frame, &mut fonts, None);
        draw(&mut ctx)
    };
    (out, frame)
}
