use std::collections::HashMap;
use std::sync::Arc;

use glyphon::fontdb::{Database, Source};
use glyphon::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache};

use crate::{Font, Weight};

const SANS: &str = "IBM Plex Sans";
const MONO: &str = "IBM Plex Mono";

const FACES: [&[u8]; 6] = [
    include_bytes!("../../../../assets/fonts/IBMPlexSans-Regular.ttf"),
    include_bytes!("../../../../assets/fonts/IBMPlexSans-Medium.ttf"),
    include_bytes!("../../../../assets/fonts/IBMPlexSans-SemiBold.ttf"),
    include_bytes!("../../../../assets/fonts/IBMPlexSans-Bold.ttf"),
    include_bytes!("../../../../assets/fonts/IBMPlexMono-Regular.ttf"),
    include_bytes!("../../../../assets/fonts/IBMPlexMono-Bold.ttf"),
];

/// The monospace cell in whole pixels.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CellSize {
    pub width: f32,
    pub height: f32,
}

/// Above this many measured strings a face gives up its widths.
const WIDTH_CACHE_CAP: usize = 4096;

/// One way of drawing text: the face and the size it is shaped at.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Face {
    pub(crate) font: Font,
    pub(crate) weight: Weight,
    pub(crate) size: u32,
}

impl Face {
    pub(crate) fn new(font: Font, weight: Weight, size: f32) -> Self {
        Self {
            font,
            weight,
            size: size.to_bits(),
        }
    }
}

/// The font database and the shaping caches.
pub struct Fonts {
    pub(crate) system: FontSystem,
    pub(crate) swash: SwashCache,
    /// What a face already measured, by the text it measured.
    widths: HashMap<Face, HashMap<Box<str>, f32>>,
}

impl Fonts {
    /// The vendored faces plus the system fonts for glyphs they lack.
    pub fn new() -> Self {
        Self::build(true)
    }

    /// The vendored faces only. Renders the same on every machine.
    pub fn embedded() -> Self {
        Self::build(false)
    }

    fn build(system_fonts: bool) -> Self {
        let mut db = Database::new();
        for face in FACES {
            db.load_font_source(Source::Binary(Arc::new(face)));
        }
        if system_fonts {
            db.load_system_fonts();
        }
        db.set_sans_serif_family(SANS);
        db.set_monospace_family(MONO);
        Self {
            system: FontSystem::new_with_locale_and_db("en-US".into(), db),
            swash: SwashCache::new(),
            widths: HashMap::new(),
        }
    }

    pub(crate) fn attrs(font: Font, weight: Weight) -> Attrs<'static> {
        let family = match font {
            Font::Sans => SANS,
            Font::Mono => MONO,
        };
        Attrs::new()
            .family(Family::Name(family))
            .weight(weight.into())
    }

    /// The advance of `text` on one line, shaped once and kept.
    pub fn measure(&mut self, text: &str, font: Font, weight: Weight, size: f32) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        let face = Face::new(font, weight, size);
        if let Some(width) = self.widths.get(&face).and_then(|face| face.get(text)) {
            return *width;
        }
        let width = self.shaped(text, font, weight, size);
        let widths = self.widths.entry(face).or_default();
        if widths.len() >= WIDTH_CACHE_CAP {
            widths.clear();
        }
        widths.insert(text.into(), width);
        width
    }

    fn shaped(&mut self, text: &str, font: Font, weight: Weight, size: f32) -> f32 {
        let mut buf = Buffer::new(&mut self.system, Metrics::new(size, size));
        buf.set_text(text, &Fonts::attrs(font, weight), Shaping::Advanced, None);
        buf.shape_until_scroll(&mut self.system, false);
        buf.layout_runs().map(|run| run.line_w).fold(0.0, f32::max)
    }

    /// The cell of the mono font at `size`: its advance and 1.35 em, both rounded.
    pub fn cell_size(&mut self, size: f32) -> CellSize {
        let advance = self.measure("M", Font::Mono, Weight::Regular, size);
        CellSize {
            width: advance.round().max(1.0),
            height: (size * 1.35).round().max(1.0),
        }
    }
}

impl Default for Fonts {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Weight> for glyphon::Weight {
    fn from(weight: Weight) -> Self {
        match weight {
            Weight::Regular => glyphon::Weight::NORMAL,
            Weight::Medium => glyphon::Weight::MEDIUM,
            Weight::SemiBold => glyphon::Weight::SEMIBOLD,
            Weight::Bold => glyphon::Weight::BOLD,
        }
    }
}
