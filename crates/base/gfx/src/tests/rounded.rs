//! Rounded quads: their corners give way, their middles stay whole, a ring holds only its edge.

use crate::{Color, Fonts, Frame, Rect, Renderer, Size};

fn drawn(draw: impl FnOnce(&mut Frame)) -> super::readback::Image {
    let size = Size::new(32, 32);
    let mut r = Renderer::headless(size, Fonts::embedded()).expect("a GPU adapter");
    let mut frame = Frame::new(size, Color::BLACK);
    draw(&mut frame);
    r.snapshot(&frame).expect("snapshot")
}

#[test]
fn a_rounded_quad_gives_up_its_corners_and_keeps_its_middle() {
    let image = drawn(|frame| frame.rounded(Rect::new(4.0, 4.0, 24.0, 24.0), Color::WHITE, 8.0));
    assert!(image.pixel(4, 4)[0] < 16, "the corner is cut away");
    assert!(image.pixel(16, 16)[0] > 240, "the middle is whole");
    assert!(
        image.pixel(16, 4)[0] > 240,
        "and so is the middle of an edge"
    );
}

#[test]
fn a_ring_draws_its_edge_and_leaves_its_inside() {
    let image = drawn(|frame| frame.ring(Rect::new(4.0, 4.0, 24.0, 24.0), Color::WHITE, 4.0, 1.0));
    assert!(image.pixel(16, 4)[0] > 200, "the edge is drawn");
    assert!(image.pixel(16, 16)[0] < 16, "the inside is not");
}

#[test]
fn a_shadow_fades_out_towards_its_edge() {
    let image = drawn(|frame| frame.shadow(Rect::new(0.0, 0.0, 32.0, 32.0), Color::WHITE, 8.0));
    let (middle, near, edge) = (
        image.pixel(16, 16)[0],
        image.pixel(16, 4)[0],
        image.pixel(16, 0)[0],
    );
    assert!(middle > 240, "the middle is whole: {middle}");
    assert!(near < middle && near > edge, "it fades: {near}");
    assert!(edge < 40, "and is nearly gone at the edge: {edge}");
}

#[test]
fn a_hatch_draws_thin_stripes_and_leaves_the_rest() {
    let image = drawn(|frame| frame.hatched(Rect::new(0.0, 0.0, 32.0, 32.0), Color::WHITE, 6.0));
    let all = (0..32).flat_map(|y| (0..32).map(move |x| (x, y)));
    let lit = all.filter(|(x, y)| image.pixel(*x, *y)[0] > 128).count();
    assert!(
        lit > 32 * 32 / 12 && lit < 32 * 32 / 3,
        "a stripe in every few pixels: {lit}"
    );
    let row: Vec<bool> = (0..12).map(|x| image.pixel(x, 0)[0] > 128).collect();
    let stripes: Vec<usize> = (0..12).filter(|x| row[*x]).collect();
    assert_eq!(stripes.len(), 2, "one stripe in every six pixels: {row:?}");
    assert_eq!(stripes[1] - stripes[0], 6, "{row:?}");
}
