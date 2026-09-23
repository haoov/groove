//! What every draw function is given: the tokens, the styles, the regions, and the frame.

use groove_controllers::AppState;
use groove_gfx::{CellGrid, CellSize, Color, Fonts, Frame, Rect, Size, TextStyle};
use groove_types::Timestamp;

use crate::hit::{Chars, Hits, Scroller, Target};
use crate::layout::Layout;
use crate::mark::Mark;
use crate::style::Styles;
use crate::tokens::Tokens;

/// What the renderer measured about the window this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub size: Size,
    pub scale: f32,
    /// What the config says to draw the interface and code with, in points.
    pub text: f32,
    pub code: f32,
    pub cell: CellSize,
    /// Milliseconds since start, for what moves.
    pub tick: u64,
    /// The wall clock, for what says how long ago.
    pub now: Timestamp,
}

impl Metrics {
    /// Every number a frame draws with, at this window's scale and sizes.
    pub fn tokens(&self) -> Tokens {
        Tokens::sized(self.scale, self.text, self.code)
    }
}

pub struct Ctx<'a> {
    pub tokens: Tokens,
    pub styles: Styles,
    pub layout: Layout,
    /// One cell of the code font, which the agent's own grid stands on.
    pub cell: CellSize,
    pub tick: u64,
    pub now: Timestamp,
    frame: &'a mut Frame,
    fonts: &'a mut Fonts,
    hits: &'a mut Hits,
    clip: Option<Rect>,
}

impl<'a> Ctx<'a> {
    pub fn new(
        app: &AppState,
        metrics: Metrics,
        layout: Layout,
        frame: &'a mut Frame,
        fonts: &'a mut Fonts,
        hits: &'a mut Hits,
    ) -> Self {
        let tokens = metrics.tokens();
        Self {
            tokens,
            styles: Styles::new(app.config.theme(), tokens),
            layout,
            cell: metrics.cell,
            tick: metrics.tick,
            now: metrics.now,
            frame,
            fonts,
            hits,
            clip: None,
        }
    }

    /// Registers `target` at `rect`. Only the part the clip leaves visible is reachable.
    pub fn hit(&mut self, rect: Rect, target: Target) {
        let rect = self.clip.map_or(rect, |clip| clip.intersect(rect));
        if rect.is_empty() {
            return;
        }
        self.hits.push(rect, target);
    }

    pub fn showing(&mut self, rows: std::ops::Range<usize>) {
        self.hits.showing(rows);
    }

    pub fn characters(&mut self, chars: Chars) {
        self.hits.characters(chars);
    }

    pub fn scrolls(&mut self, which: Scroller, extent: f32) {
        self.hits.scrolls(which, extent);
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
        let outer = self.clip;
        self.clip = Some(outer.map_or(rect, |clip| clip.intersect(rect)));
        self.frame.push_clip(rect);
        draw(self);
        self.frame.pop_clip();
        self.clip = outer;
    }

    /// The width this text takes in this style.
    pub fn measure(&mut self, text: &str, style: &TextStyle) -> f32 {
        self.fonts
            .measure(text, style.font, style.weight, style.size)
    }
}
