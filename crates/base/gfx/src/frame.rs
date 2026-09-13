use crate::{CellGrid, Color, Rect, Size};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Font {
    Sans,
    Mono,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Weight {
    Regular,
    Medium,
    SemiBold,
    Bold,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TextStyle {
    pub font: Font,
    pub weight: Weight,
    pub size: f32,
    pub color: Color,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Quad {
    pub rect: Rect,
    pub color: Color,
    pub clip: Rect,
}

/// One line of chrome text, vertically centred in `height`.
#[derive(Clone, PartialEq, Debug)]
pub struct TextRun {
    pub x: f32,
    pub y: f32,
    pub height: f32,
    pub text: String,
    pub style: TextStyle,
    pub clip: Rect,
}

/// Quads draw first, then text. A later layer draws over an earlier one.
#[derive(Default)]
pub(crate) struct Layer {
    pub quads: Vec<Quad>,
    pub texts: Vec<TextRun>,
    pub grids: Vec<CellGrid>,
}

/// The display list for one frame.
pub struct Frame {
    pub size: Size,
    pub background: Color,
    pub(crate) layers: Vec<Layer>,
    clips: Vec<Rect>,
}

impl Frame {
    pub fn new(size: Size, background: Color) -> Self {
        Self {
            size,
            background,
            layers: vec![Layer::default()],
            clips: Vec::new(),
        }
    }

    /// Everything emitted after this draws over what came before.
    pub fn layer(&mut self) {
        self.layers.push(Layer::default());
    }

    pub fn clip(&self) -> Rect {
        self.clips.last().copied().unwrap_or(self.size.rect())
    }

    /// Runs `f` with drawing clipped to `rect`, inside the current clip.
    pub fn clipped(&mut self, rect: Rect, f: impl FnOnce(&mut Frame)) {
        self.clips.push(self.clip().intersect(rect));
        f(self);
        self.clips.pop();
    }

    pub fn quad(&mut self, rect: Rect, color: Color) {
        if rect.is_empty() || color.is_transparent() {
            return;
        }
        let clip = self.clip();
        self.top().quads.push(Quad { rect, color, clip });
    }

    /// A one-pixel frame just inside `rect`.
    pub fn border(&mut self, rect: Rect, color: Color) {
        self.quad(Rect::new(rect.x, rect.y, rect.w, 1.0), color);
        self.quad(Rect::new(rect.x, rect.bottom() - 1.0, rect.w, 1.0), color);
        self.quad(Rect::new(rect.x, rect.y, 1.0, rect.h), color);
        self.quad(Rect::new(rect.right() - 1.0, rect.y, 1.0, rect.h), color);
    }

    pub fn text(&mut self, text: impl Into<String>, x: f32, y: f32, height: f32, style: TextStyle) {
        let text = text.into();
        if text.is_empty() {
            return;
        }
        let clip = self.clip();
        self.top().texts.push(TextRun {
            x,
            y,
            height,
            text,
            style,
            clip,
        });
    }

    pub fn grid(&mut self, mut grid: CellGrid) {
        grid.clip = self.clip();
        self.top().grids.push(grid);
    }

    fn top(&mut self) -> &mut Layer {
        self.layers
            .last_mut()
            .unwrap_or_else(|| unreachable!("a frame always has a layer"))
    }
}
