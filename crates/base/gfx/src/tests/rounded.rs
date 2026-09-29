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
