//! How a quad is cut.

/// How a quad is cut: its corners' radius, the width of its edge when only that is drawn, its fade.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Shape {
    pub radius: f32,
    pub stroke: f32,
    /// How far the quad fades out inside its edge; none keeps it sharp.
    pub blur: f32,
    /// The gap between diagonal stripes, drawn instead of a fill; none fills it.
    pub hatch: f32,
    /// The corner radius of hexes whose edges are drawn, `stroke` wide, instead of a fill,
    /// counted from the window's corner; none fills it. A fade keeps those near the middle.
    pub lattice: f32,
}
