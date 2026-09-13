use std::collections::HashMap;

use glyphon::{
    Buffer, Cache, Metrics, Resolution, Shaping, TextArea, TextAtlas, TextBounds, TextRenderer,
    Viewport,
};

use crate::fonts::CellSize;
use crate::{Color, Font, Fonts, Rect, Result, Size, TextRun, Weight};

const GLYPH_CACHE_CAP: usize = 4096;

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

/// One shaped line of chrome text.
struct Run {
    buffer: Buffer,
    x: f32,
    y: f32,
    color: Color,
    clip: Rect,
}

/// All text of the frame: one atlas, one renderer per layer.
pub(crate) struct TextPass {
    atlas: TextAtlas,
    viewport: Viewport,
    renderers: Vec<TextRenderer>,
    glyphs: HashMap<GlyphKey, Buffer>,
    runs: Vec<Vec<Run>>,
    placements: Vec<Vec<Placement>>,
}

impl TextPass {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let cache = Cache::new(device);
        Self {
            atlas: TextAtlas::new(device, queue, &cache, format),
            viewport: Viewport::new(device, &cache),
            renderers: Vec::new(),
            glyphs: HashMap::new(),
            runs: Vec::new(),
            placements: Vec::new(),
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
    }

    pub fn push_run(&mut self, layer: usize, fonts: &mut Fonts, run: &TextRun) {
        let style = run.style;
        let mut buffer = Buffer::new(&mut fonts.system, Metrics::new(style.size, run.height));
        buffer.set_size(None, Some(run.height));
        let attrs = Fonts::attrs(style.font, style.weight);
        buffer.set_text(&run.text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut fonts.system, false);
        self.runs[layer].push(Run {
            buffer,
            x: run.x,
            y: run.y,
            color: style.color,
            clip: run.clip,
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
        self.glyphs
            .entry(key)
            .or_insert_with(|| shape_glyph(fonts, ch, bold, size, cell));
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
        let layers = self.runs.len();
        for (layer, renderer) in self.renderers.iter_mut().enumerate().take(layers) {
            let runs = self.runs[layer]
                .iter()
                .map(|r| area(&r.buffer, r.x, r.y, r.color, r.clip));
            let glyphs = &self.glyphs;
            let placed = self.placements[layer].iter().filter_map(|p| {
                glyphs
                    .get(&p.key)
                    .map(|b| area(b, p.x, p.y, p.color, p.clip))
            });
            renderer.prepare(
                device,
                queue,
                &mut fonts.system,
                &mut self.atlas,
                &self.viewport,
                runs.chain(placed),
                &mut fonts.swash,
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

/// One glyph in a box two cells wide, so a wide character is not wrapped.
fn shape_glyph(fonts: &mut Fonts, ch: char, bold: bool, size: f32, cell: CellSize) -> Buffer {
    let mut buffer = Buffer::new(&mut fonts.system, Metrics::new(size, cell.height));
    buffer.set_size(Some(cell.width * 2.0), Some(cell.height));
    let weight = if bold { Weight::Bold } else { Weight::Regular };
    let mut text = [0u8; 4];
    let text = ch.encode_utf8(&mut text);
    buffer.set_text(
        text,
        &Fonts::attrs(Font::Mono, weight),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(&mut fonts.system, false);
    buffer
}

fn area(buffer: &Buffer, x: f32, y: f32, color: Color, clip: Rect) -> TextArea<'_> {
    TextArea {
        buffer,
        left: x,
        top: y,
        scale: 1.0,
        bounds: TextBounds {
            left: clip.x.floor() as i32,
            top: clip.y.floor() as i32,
            right: clip.right().ceil() as i32,
            bottom: clip.bottom().ceil() as i32,
        },
        default_color: color.glyphon(),
        custom_glyphs: &[],
    }
}
