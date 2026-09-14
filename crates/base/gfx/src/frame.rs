use crate::{CellGrid, Color, Icon, Rect, Size};

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

/// One icon, drawn in its own box.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct IconDraw {
    pub rect: Rect,
    pub icon: Icon,
    /// Eighths of a turn, clockwise.
    pub turn: u8,
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
pub struct Layer {
    pub quads: Vec<Quad>,
    pub texts: Vec<TextRun>,
    pub grids: Vec<CellGrid>,
    pub icons: Vec<IconDraw>,
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

    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    pub fn clip(&self) -> Rect {
        self.clips.last().copied().unwrap_or(self.size.rect())
    }

    /// Narrows the clip to `rect` until `pop_clip`.
    pub fn push_clip(&mut self, rect: Rect) {
        self.clips.push(self.clip().intersect(rect));
    }

    pub fn pop_clip(&mut self) {
        self.clips.pop();
    }

    /// Runs `f` with drawing clipped to `rect`, inside the current clip.
    pub fn clipped(&mut self, rect: Rect, f: impl FnOnce(&mut Frame)) {
        self.push_clip(rect);
        f(self);
        self.pop_clip();
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

    /// An icon in `rect`, upright.
    pub fn icon(&mut self, rect: Rect, icon: Icon, color: Color) {
        self.icon_turned(rect, icon, 0, color);
    }

    /// An icon in `rect`, turned by eighths of a turn.
    pub fn icon_turned(&mut self, rect: Rect, icon: Icon, turn: u8, color: Color) {
        if rect.is_empty() || color.is_transparent() {
            return;
        }
        let clip = self.clip();
        self.top().icons.push(IconDraw {
            rect,
            icon,
            turn,
            color,
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
