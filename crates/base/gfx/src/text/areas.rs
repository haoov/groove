//! One area of text as glyphon takes it: a run, a placed glyph, an icon's mask.

use glyphon::{
    Buffer, ContentType, Metrics, RasterizeCustomGlyphRequest, RasterizedCustomGlyph, Shaping,
    TextArea, TextBounds,
};

use super::IconArea;
use crate::fonts::CellSize;
use crate::icons::Icons;
use crate::{Color, Font, Fonts, Rect, Weight};

/// One glyph in a box two cells wide, for a wide character.
pub(super) fn shape_glyph(
    fonts: &mut Fonts,
    ch: char,
    bold: bool,
    size: f32,
    cell: CellSize,
) -> Buffer {
    let mut buffer = Buffer::new(&mut fonts.system, Metrics::new(size, cell.height));
    buffer.set_size(Some(cell.width * 2.0), Some(cell.height));
    let weight = if bold { Weight::Bold } else { Weight::Regular };
    let mut text = [0u8; 4];
    let text = ch.encode_utf8(&mut text);
    let attrs = fonts.attrs(Font::Mono, weight);
    buffer.set_text(text, &attrs, Shaping::Advanced, None);
    buffer.shape_until_scroll(&mut fonts.system, false);
    buffer
}

/// One icon as an area of its own, on a buffer with no text in it.
pub(super) fn mark<'a>(icon: &'a IconArea, empty: &'a Buffer) -> TextArea<'a> {
    TextArea {
        buffer: empty,
        left: 0.0,
        top: 0.0,
        scale: 1.0,
        bounds: bounds(icon.clip),
        default_color: Color::WHITE.glyphon(),
        custom_glyphs: &icon.glyph,
    }
}

/// The mask one icon's glyph draws with, from the registry that owns it.
pub(super) fn rasterize(
    icons: &Icons,
    request: RasterizeCustomGlyphRequest,
) -> Option<RasterizedCustomGlyph> {
    icons
        .rasterize(request.id, request.width, request.height)
        .map(|data| RasterizedCustomGlyph {
            data,
            content_type: ContentType::Mask,
        })
}

pub(super) fn area(buffer: &Buffer, x: f32, y: f32, color: Color, clip: Rect) -> TextArea<'_> {
    TextArea {
        buffer,
        left: x,
        top: y,
        scale: 1.0,
        bounds: bounds(clip),
        default_color: color.glyphon(),
        custom_glyphs: &[],
    }
}

pub(super) fn bounds(clip: Rect) -> TextBounds {
    TextBounds {
        left: clip.x.floor() as i32,
        top: clip.y.floor() as i32,
        right: clip.right().ceil() as i32,
        bottom: clip.bottom().ceil() as i32,
    }
}
