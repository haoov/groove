//! A line's setters, each giving it one more thing to draw.

use groove_gfx::Color;
use groove_types::{Highlight, Severity};

use super::{Line, Tile};

impl<'a> Line<'a> {
    /// A row of code a note stands on.
    pub fn tile(mut self, tile: Option<(Tile, Option<String>)>) -> Self {
        self.tile = tile;
        self
    }

    pub fn noted(mut self, noted: bool) -> Self {
        self.noted = noted;
        self
    }

    /// A row that spans the width, centred, on its own ground.
    pub fn banner(text: &'a str) -> Self {
        Self {
            banner: true,
            ..Self::new(text)
        }
    }

    pub fn empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    pub fn gutters(mut self, gutters: &'a [&'a str]) -> Self {
        self.gutters = gutters;
        self
    }

    pub fn spans(mut self, spans: &'a [Highlight]) -> Self {
        self.spans = spans;
        self
    }

    pub fn ground(mut self, ground: Color) -> Self {
        self.ground = ground.into();
        self
    }

    pub fn gutter(mut self, gutter: Option<Color>) -> Self {
        self.gutter = gutter;
        self
    }

    pub fn mark(mut self, mark: Option<Color>) -> Self {
        self.mark = mark;
        self
    }

    pub fn caret(mut self, caret: Option<usize>) -> Self {
        self.caret = caret;
        self
    }

    pub fn held(mut self, held: Option<(usize, usize, bool)>) -> Self {
        self.held = held;
        self
    }

    pub fn found(mut self, found: &'a [(usize, usize)]) -> Self {
        self.found = found;
        self
    }

    pub fn words(mut self, words: &'a [(usize, usize)], word: Option<Color>) -> Self {
        self.words = words;
        self.word = word;
        self
    }

    pub fn blame(mut self, blame: Option<(&'a str, Option<&'a str>)>) -> Self {
        self.blame = blame;
        self
    }

    pub fn standing(mut self, standing: Option<(usize, usize)>) -> Self {
        self.standing = standing;
        self
    }

    pub fn faults(mut self, faults: &'a [(usize, usize, Severity)]) -> Self {
        self.faults = faults;
        self
    }
}
