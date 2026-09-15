//! What every draw function is given: the tokens, the styles, the regions, and the frame.

use groove_controllers::AppState;
use groove_gfx::{CellGrid, CellSize, Color, Fonts, Frame, Rect, Size, TextStyle};

use crate::hit::{Hits, Target};
use crate::layout::Layout;
use crate::mark::Mark;
use crate::style::Styles;
use crate::tokens::Tokens;

/// What the renderer measured about the window this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub size: Size,
    pub scale: f32,
    pub cell: CellSize,
    /// Milliseconds since start, for what moves.
    pub tick: u64,
}

pub struct Ctx<'a> {
    pub tokens: Tokens,
    pub styles: Styles,
    pub layout: Layout,
    pub tick: u64,
    frame: &'a mut Frame,
    fonts: &'a mut Fonts,
    hits: &'a mut Hits,
}

impl<'a> Ctx<'a> {
    pub fn new(
        app: &AppState,
        metrics: Metrics,
        frame: &'a mut Frame,
        fonts: &'a mut Fonts,
        hits: &'a mut Hits,
    ) -> Self {
        let tokens = Tokens::new(metrics.scale);
        Self {
            tokens,
            styles: Styles::new(app.config.theme(), tokens),
            layout: Layout::new(metrics.size, &tokens),
            tick: metrics.tick,
            frame,
            fonts,
            hits,
        }
    }

    /// Says that `target` was drawn in `rect`, for the pointer to find.
    pub fn hit(&mut self, rect: Rect, target: Target) {
        self.hits.push(rect, target);
    }

    pub fn quad(&mut self, rect: Rect, color: Color) {
        self.frame.quad(rect, color);
    }

    pub fn border(&mut self, rect: Rect, color: Color) {
        self.frame.border(rect, color);
    }

    pub fn text(&mut self, text: &str, x: f32, y: f32, height: f32, style: TextStyle) {
        self.frame.text(text, x, y, height, style);
    }

    /// A mark in `rect`, turned by eighths of a turn.
    pub fn icon(&mut self, rect: Rect, mark: Mark, turn: u8, color: Color) {
        self.frame.icon_turned(rect, mark.shape(), turn, color);
    }

    pub fn grid(&mut self, grid: CellGrid) {
        self.frame.grid(grid);
    }

    /// Everything after this draws over what came before.
    pub fn layer(&mut self) {
        self.frame.layer();
    }

    /// Runs `draw` with everything clipped to `rect`.
    pub fn clipped(&mut self, rect: Rect, draw: impl FnOnce(&mut Self)) {
        self.frame.push_clip(rect);
        draw(self);
        self.frame.pop_clip();
    }

    /// The width this text takes in this style.
    pub fn measure(&mut self, text: &str, style: &TextStyle) -> f32 {
        self.fonts
            .measure(text, style.font, style.weight, style.size)
    }
}
