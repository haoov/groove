//! One way out of the process: the GPU. A `Frame` goes in, a window presents.

mod backdrop;
mod boxdraw;
mod cell;
mod color;
mod error;
mod fonts;
mod frame;
mod geom;
mod glass;
mod gpu;
mod grid;
mod icons;
mod kept;
mod passes;
mod quads;
mod renderer;
mod text;
mod theme;

#[cfg(test)]
mod tests;

pub use cell::{Cell, CellGrid, WIDE_SPACER};
pub use color::Color;
pub use error::{Error, Result};
pub use fonts::{CellSize, Family, Fonts};
pub use frame::{Font, Frame, Glass, Quad, Shape, TextRun, TextStyle, Weight};
pub use geom::{Align, Edges, Rect, Size};
pub use icons::Icon;
pub use renderer::Renderer;
pub use theme::Palette;
