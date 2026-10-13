//! The text pass's caches cut back to what the last frame drew, once they pass their cap.

use std::collections::{HashMap, HashSet};

use super::{GLYPH_CACHE_CAP, RUN_CACHE_CAP, TextPass};

impl TextPass {
    /// Called before the last frame's runs are let go.
    pub(super) fn keep_recent(&mut self) {
        if self.glyphs.len() > GLYPH_CACHE_CAP {
            let drawn: HashSet<_> = self
                .placements
                .iter()
                .flatten()
                .map(|one| one.key)
                .collect();
            self.glyphs.retain(|key, _| drawn.contains(key));
        }
        if self.lines.len() > RUN_CACHE_CAP {
            self.keep_drawn_lines();
        }
    }

    fn keep_drawn_lines(&mut self) {
        let drawn: HashSet<usize> = self.runs.iter().flatten().map(|run| run.at).collect();
        let mut moved: HashMap<usize, usize> = HashMap::new();
        for (at, buffer) in std::mem::take(&mut self.lines).into_iter().enumerate() {
            if drawn.contains(&at) {
                moved.insert(at, self.lines.len());
                self.lines.push(buffer);
            }
        }
        for texts in self.index.values_mut() {
            texts.retain(|_, at| match moved.get(at) {
                Some(now) => {
                    *at = *now;
                    true
                }
                None => false,
            });
        }
        self.index.retain(|_, texts| !texts.is_empty());
    }
}
