//! An icon or a terminal cell's glyph put on a layer of the pass.

use glyphon::CustomGlyph;

use super::areas::shape_glyph;
use super::{GlyphKey, IconArea, Placement, TextPass};
use crate::fonts::CellSize;
use crate::frame::IconDraw;
use crate::{Color, Fonts, Rect};

impl TextPass {
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
}
