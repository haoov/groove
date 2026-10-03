//! The app's half of the draw context: its layout, and the hits the input reads.

use groove_gfx::Rect;

use crate::hit::{Hits, Target};
use crate::layout::Layout;
use groove_ui_kit::base::ctx::App;

pub struct Drawn<'a> {
    pub layout: Layout,
    pub hits: &'a mut Hits,
}

impl App for Drawn<'_> {
    type Target = Target;

    fn hit(&mut self, rect: Rect, target: Target) {
        self.hits.push(rect, target);
    }

    fn caret(&mut self, rect: Rect) {
        self.hits.careted(rect);
    }
}

pub type Ctx<'a> = groove_ui_kit::base::ctx::Ctx<'a, Drawn<'a>>;
