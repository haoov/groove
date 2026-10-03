//! What every draw function is given: the tokens, the styles, the regions, and the frame.

use groove_gfx::{CellGrid, CellSize, Color, Fonts, Frame, Rect, Size, TextStyle};
use groove_types::{ThemeName, Timestamp};

use crate::base::mark::Mark;
use crate::base::style::Styles;
use crate::base::tokens::Tokens;

/// What the renderer measured about the window this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub size: Size,
    pub scale: f32,
    /// What the config says to draw the interface and code with, in points.
    pub text: f32,
    pub code: f32,
    pub terminal: f32,
    /// One cell of the terminal font.
    pub cell: CellSize,
    /// One character's width of the code font.
    pub advance: f32,
    /// Milliseconds since start.
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

/// The app's half of a frame: what a hit names, and where it is kept.
pub trait App {
    type Target: Clone + PartialEq;

    fn hit(&mut self, rect: Rect, target: Self::Target);
}

pub struct Ctx<'a, A: App> {
    pub tokens: Tokens,
    pub styles: Styles,
    /// The whole window, which a modal centres in.
    pub window: Rect,
    /// One cell of the terminal font, which every terminal's grid stands on.
    pub cell: CellSize,
    /// The terminal font's size, scaled.
    pub terminal: f32,
    /// One character's width of the code font.
    pub advance: f32,
    pub tick: u64,
    pub now: Timestamp,
    pub app: A,
    frame: &'a mut Frame,
    fonts: &'a mut Fonts,
    hover: Option<A::Target>,
    clip: Option<Rect>,
}

impl<'a, A: App> Ctx<'a, A> {
    pub fn new(
        theme: ThemeName,
        metrics: Metrics,
        app: A,
        frame: &'a mut Frame,
        fonts: &'a mut Fonts,
        hover: Option<A::Target>,
    ) -> Self {
        let tokens = metrics.tokens();
        Self {
            tokens,
            styles: Styles::new(theme, tokens),
            window: metrics.size.rect(),
            cell: metrics.cell,
            terminal: metrics.terminal * metrics.scale,
            advance: metrics.advance,
            tick: metrics.tick,
            now: metrics.now,
            app,
            frame,
            fonts,
            hover,
            clip: None,
        }
    }

    pub fn hover(&self) -> Option<&A::Target> {
        self.hover.as_ref()
    }

    pub fn hovered(&self, target: &A::Target) -> bool {
        self.hover.as_ref() == Some(target)
    }

    /// Registers `target` at `rect`; returns whether the pointer rests on it.
    pub fn interact(&mut self, rect: Rect, target: A::Target) -> bool {
        let on = self.hovered(&target);
        self.hit(rect, target);
        on
    }

    /// Registers `target` at `rect`. Only the part the clip leaves visible is reachable.
    pub fn hit(&mut self, rect: Rect, target: A::Target) {
        let rect = self.clip.map_or(rect, |clip| clip.intersect(rect));
        if !rect.is_empty() {
            self.app.hit(rect, target);
        }
    }

    pub fn quad(&mut self, rect: Rect, color: Color) {
        self.frame.quad(rect, color);
    }

    pub fn border(&mut self, rect: Rect, color: Color) {
        self.frame.border(rect, color);
    }

    pub fn rounded(&mut self, rect: Rect, color: Color, radius: f32) {
        self.frame.rounded(rect, color, radius);
    }

    pub fn ring(&mut self, rect: Rect, color: Color, radius: f32, stroke: f32) {
        self.frame.ring(rect, color, radius, stroke);
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
