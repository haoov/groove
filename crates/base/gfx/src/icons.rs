//! Icons: an SVG rasterised into the glyph atlas at the size it is drawn at.

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};

/// A mark the app draws. The shape only; what it means is the ui's business.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Flag,
    Compass,
    Eye,
    Kanban,
    Gear,
    /// Three quarters of a ring, for what is in flight.
    Notch,
    Cube,
    ArrowUp,
    ArrowDown,
    Plus,
    Dot,
    CaretDown,
    Sidebar,
}

impl Icon {
    /// Rotations an icon can be drawn at: eighths of a turn.
    pub const TURNS: u8 = 8;

    pub(crate) const ALL: [Icon; 13] = [
        Icon::Flag,
        Icon::Compass,
        Icon::Eye,
        Icon::Kanban,
        Icon::Gear,
        Icon::Notch,
        Icon::Cube,
        Icon::ArrowUp,
        Icon::ArrowDown,
        Icon::Plus,
        Icon::Dot,
        Icon::CaretDown,
        Icon::Sidebar,
    ];

    fn svg(self) -> &'static [u8] {
        match self {
            Icon::Flag => include_bytes!("../../../../assets/icons/flag.svg"),
            Icon::Compass => include_bytes!("../../../../assets/icons/compass.svg"),
            Icon::Eye => include_bytes!("../../../../assets/icons/eye.svg"),
            Icon::Kanban => include_bytes!("../../../../assets/icons/kanban.svg"),
            Icon::Gear => include_bytes!("../../../../assets/icons/gear.svg"),
            Icon::Notch => include_bytes!("../../../../assets/icons/circle-notch.svg"),
            Icon::Cube => include_bytes!("../../../../assets/icons/cube.svg"),
            Icon::ArrowUp => include_bytes!("../../../../assets/icons/arrow-up.svg"),
            Icon::ArrowDown => include_bytes!("../../../../assets/icons/arrow-down.svg"),
            Icon::Plus => include_bytes!("../../../../assets/icons/plus.svg"),
            Icon::Dot => include_bytes!("../../../../assets/icons/circle-fill.svg"),
            Icon::CaretDown => include_bytes!("../../../../assets/icons/caret-down.svg"),
            Icon::Sidebar => include_bytes!("../../../../assets/icons/sidebar.svg"),
        }
    }

    /// The atlas' name for this icon at this rotation.
    pub(crate) fn glyph_id(self, turn: u8) -> u16 {
        let at = Self::ALL.iter().position(|icon| *icon == self).unwrap_or(0) as u16;
        at * turns() + u16::from(turn % Self::TURNS)
    }
}

/// Every icon, parsed once and kept for the life of the renderer.
pub(crate) struct Icons {
    trees: Vec<Tree>,
}

impl Icons {
    pub fn new() -> Self {
        let options = Options::default();
        let trees = Icon::ALL
            .iter()
            .filter_map(|icon| Tree::from_data(icon.svg(), &options).ok())
            .collect();
        Self { trees }
    }

    /// The icon's coverage at this size, one byte a pixel, for the atlas.
    pub fn rasterize(&self, glyph: u16, width: u16, height: u16) -> Option<Vec<u8>> {
        let tree = self.trees.get((glyph / turns()) as usize)?;
        let (w, h) = (f32::from(width), f32::from(height));
        let mut pixmap = Pixmap::new(u32::from(width), u32::from(height))?;
        let size = tree.size();
        let degrees = f32::from(glyph % turns()) * (360.0 / f32::from(Icon::TURNS));
        let transform = Transform::from_scale(w / size.width(), h / size.height()).post_rotate_at(
            degrees,
            w / 2.0,
            h / 2.0,
        );
        resvg::render(tree, transform, &mut pixmap.as_mut());
        Some(pixmap.data().iter().skip(3).step_by(4).copied().collect())
    }
}

fn turns() -> u16 {
    u16::from(Icon::TURNS)
}
