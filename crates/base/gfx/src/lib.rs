//! One way out of the process: the GPU. A `Frame` goes in, a window presents.

mod boxdraw;
mod cell;
mod color;
mod error;
mod fonts;
mod frame;
mod geom;
mod gpu;
mod grid;
mod quads;
mod renderer;
mod text;
mod theme;

#[cfg(test)]
mod tests;

pub use cell::{Cell, CellGrid, WIDE_SPACER};
pub use color::Color;
pub use error::{Error, Result};
pub use fonts::{CellSize, Fonts};
pub use frame::{Font, Frame, Quad, TextRun, TextStyle, Weight};
pub use geom::{Rect, Size};
pub use renderer::Renderer;
pub use theme::{Palette, Theme};
pub use wgpu::SurfaceTarget;
