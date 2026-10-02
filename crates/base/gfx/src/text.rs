use std::collections::HashMap;

use glyphon::{
    Buffer, Cache, CustomGlyph, Metrics, Resolution, Shaping, TextAtlas, TextRenderer, Viewport,
};

mod areas;

use self::areas::{area, mark, rasterize, shape_glyph};
use crate::fonts::{CellSize, Face, one_paragraph};
use crate::frame::IconDraw;
use crate::icons::Icons;
use crate::{Color, Fonts, Rect, Result, Size, TextRun};

const GLYPH_CACHE_CAP: usize = 4096;

/// Above this many shaped lines the pass gives them up.
const RUN_CACHE_CAP: usize = 2048;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphKey {
    ch: char,
    bold: bool,
    size: u32,
}

/// One cached glyph, placed.
struct Placement {
    key: GlyphKey,
    x: f32,
    y: f32,
    color: Color,
    clip: Rect,
}

/// One icon placed, in the shape glyphon takes. The array backs the area's slice.
struct IconArea {
    glyph: [CustomGlyph; 1],
    clip: Rect,
}

/// One line of text placed, pointing at the shaped line it draws.
struct Run {
    at: usize,
    x: f32,
    y: f32,
    color: Color,
    clip: Rect,
}

/// How a line was shaped: its face, and the row height it was laid out in.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Shaped {
    face: Face,
    height: u32,
}

/// What shaping a frame left behind, for a test or a debug line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cached {
    /// Lines of chrome text held, shaped whole.
    pub lines: usize,
    /// Single glyphs held, for the terminal grid.
    pub glyphs: usize,
    /// Cache misses since the window opened.
    pub shaped: u64,
}

/// All text of the frame: one atlas, one renderer per layer.
pub(crate) struct TextPass {
    atlas: TextAtlas,
    viewport: Viewport,
    renderers: Vec<TextRenderer>,
    glyphs: HashMap<GlyphKey, Buffer>,
    /// Every line shaped so far, and where each one sits.
    lines: Vec<Buffer>,
    index: HashMap<Shaped, HashMap<Box<str>, usize>>,
    runs: Vec<Vec<Run>>,
    placements: Vec<Vec<Placement>>,
    icons: Vec<Vec<IconArea>>,
    registry: Icons,
    /// What an icon-only area hangs on: a buffer with no text.
    empty: Option<Buffer>,
    shaped: u64,
}

impl TextPass {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let cache = Cache::new(device);
        Self {
            atlas: TextAtlas::new(device, queue, &cache, format),
            viewport: Viewport::new(device, &cache),
            renderers: Vec::new(),
            glyphs: HashMap::new(),
            lines: Vec::new(),
            index: HashMap::new(),
            runs: Vec::new(),
            placements: Vec::new(),
            icons: Vec::new(),
            registry: Icons::new(),
            empty: None,
            shaped: 0,
        }
    }

    pub fn begin(&mut self, layers: usize, device: &wgpu::Device, queue: &wgpu::Queue, size: Size) {
        self.viewport.update(
            queue,
            Resolution {
                width: size.width,
                height: size.height,
            },
        );
        self.runs.clear();
        self.runs.resize_with(layers, Vec::new);
        self.placements.clear();
        self.placements.resize_with(layers, Vec::new);
        self.icons.clear();
        self.icons.resize_with(layers, Vec::new);
        while self.renderers.len() < layers {
            let renderer = TextRenderer::new(
                &mut self.atlas,
                device,
                wgpu::MultisampleState::default(),
                None,
            );
            self.renderers.push(renderer);
        }
        if self.glyphs.len() > GLYPH_CACHE_CAP {
            self.glyphs.clear();
        }
        if self.lines.len() > RUN_CACHE_CAP {
            self.lines.clear();
            self.index.clear();
        }
    }

    /// What it has shaped so far and kept.
    pub fn cached(&self) -> Cached {
        Cached {
            lines: self.lines.len(),
            glyphs: self.glyphs.len(),
            shaped: self.shaped,
        }
    }

    pub fn push_run(&mut self, layer: usize, fonts: &mut Fonts, run: &TextRun) {
        let at = self.shape(fonts, run);
        self.runs[layer].push(Run {
            at,
            x: run.x,
            y: run.y,
            color: run.style.color,
            clip: run.clip,
        });
    }

    /// Where the run's shaped line sits, shaping it the first time it is asked for.
    fn shape(&mut self, fonts: &mut Fonts, run: &TextRun) -> usize {
        let style = run.style;
        let key = Shaped {
            face: Face::new(style.font, style.weight, style.size),
            height: run.height.to_bits(),
        };
        if let Some(at) = self.index.get(&key).and_then(|lines| lines.get(&*run.text)) {
            return *at;
        }
        let mut buffer = Buffer::new(&mut fonts.system, Metrics::new(style.size, run.height));
        buffer.set_size(None, Some(run.height));
        let attrs = Fonts::attrs(style.font, style.weight);
        buffer.set_text(&one_paragraph(&run.text), &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut fonts.system, false);
        self.shaped += 1;
        self.lines.push(buffer);
        let at = self.lines.len() - 1;
        self.index
            .entry(key)
            .or_default()
            .insert(run.text.as_str().into(), at);
        at
    }

    pub fn push_icon(&mut self, layer: usize, draw: &IconDraw) {
        let glyph = CustomGlyph {
            id: draw.icon.glyph_id(draw.turn),
            left: draw.rect.x,
            top: draw.rect.y,
            width: draw.rect.w,
            height: draw.rect.h,
            color: Some(draw.color.glyphon()),
            snap_to_physical_pixel: true,
            metadata: 0,
        };
        self.icons[layer].push(IconArea {
            glyph: [glyph],
            clip: draw.clip,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn push_glyph(
        &mut self,
        layer: usize,
        fonts: &mut Fonts,
        ch: char,
        bold: bool,
        size: f32,
        cell: CellSize,
        x: f32,
        y: f32,
        color: Color,
        clip: Rect,
    ) {
        let key = GlyphKey {
            ch,
            bold,
            size: size.to_bits(),
        };
        if let std::collections::hash_map::Entry::Vacant(slot) = self.glyphs.entry(key) {
            slot.insert(shape_glyph(fonts, ch, bold, size, cell));
            self.shaped += 1;
        }
        self.placements[layer].push(Placement {
            key,
            x,
            y,
            color,
            clip,
        });
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        fonts: &mut Fonts,
    ) -> Result<()> {
        let Self {
            atlas,
            viewport,
            renderers,
            glyphs,
            lines: shaped_lines,
            index: _,
            shaped: _,
            runs,
            placements,
            icons,
            registry,
            empty,
        } = self;
        let empty =
            empty.get_or_insert_with(|| Buffer::new(&mut fonts.system, Metrics::new(1.0, 1.0)));
        for (layer, renderer) in renderers.iter_mut().enumerate().take(runs.len()) {
            let lines = runs[layer]
                .iter()
                .map(|run| area(&shaped_lines[run.at], run.x, run.y, run.color, run.clip));
            let placed = placements[layer].iter().filter_map(|p| {
                glyphs
                    .get(&p.key)
                    .map(|b| area(b, p.x, p.y, p.color, p.clip))
            });
            let marks = icons[layer].iter().map(|icon| mark(icon, empty));
            renderer.prepare_with_depth_and_custom(
                device,
                queue,
                &mut fonts.system,
                atlas,
                viewport,
                lines.chain(placed).chain(marks),
                &mut fonts.swash,
                |_| 0.0,
                |request| rasterize(registry, request),
            )?;
        }
        Ok(())
    }

    pub fn draw(&self, layer: usize, pass: &mut wgpu::RenderPass<'_>) -> Result<()> {
        self.renderers[layer].render(&self.atlas, &self.viewport, pass)?;
        Ok(())
    }

    pub fn end(&mut self) {
        self.atlas.trim();
    }
}
