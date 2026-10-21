use super::golden::{compare, renderer};
use crate::{Color, Font, Frame, Palette, Rect, Size, TextStyle, Weight};

/// Vertical stripes `width` pixels wide, alternating `a` and `b`, across the frame.
fn striped(size: Size, width: f32, a: Color, b: Color) -> Frame {
    let mut frame = Frame::new(size, a);
    let mut x = width;
    while x < size.width as f32 {
        frame.quad(Rect::new(x, 0.0, width, size.height as f32), b);
        x += 2.0 * width;
    }
    frame
}

#[test]
fn the_backdrop_starts_at_the_first_layer_with_glass() {
    let mut frame = Frame::new(Size::new(8, 8), Color::BLACK);
    assert_eq!(frame.backdrop(), None);
    frame.layer();
    frame.layer();
    frame.glass(Rect::new(0.0, 0.0, 4.0, 4.0), (2.0, 1.0), Color::WHITE);
    frame.layer();
    frame.glass(Rect::new(0.0, 0.0, 4.0, 4.0), (2.0, 1.0), Color::WHITE);
    assert_eq!(frame.backdrop(), Some(2));
}

#[test]
fn a_frame_without_glass_draws_in_one_pass() {
    let size = Size::new(32, 32);
    let mut r = renderer(size);
    let frame = striped(size, 4.0, Color::BLACK, Color::WHITE);
    r.snapshot(&frame).expect("snapshot");
    assert!(r.backdrop.targets().is_none());
}

#[test]
fn glass_blurs_what_lies_below_it_and_nothing_beside_it() {
    let size = Size::new(160, 80);
    let mut r = renderer(size);
    let mut frame = striped(size, 4.0, Color::BLACK, Color::WHITE);
    frame.layer();
    frame.glass(
        Rect::new(40.0, 16.0, 80.0, 48.0),
        (6.0, 8.0),
        Color::TRANSPARENT,
    );
    let image = r.snapshot(&frame).expect("snapshot");
    assert!(r.backdrop.targets().is_some());
    let spread = |x: u32| image.pixel(x, 40)[0].abs_diff(image.pixel(x + 4, 40)[0]);
    let (beside, under) = (spread(10), spread(78));
    assert!(beside > 200, "beside: {beside}");
    assert!(under * 2 < beside, "under: {under}");
}

#[test]
fn glass_matches_golden() {
    let size = Size::new(240, 120);
    let p = Palette::MOCHA;
    let mut frame = striped(size, 6.0, p.base, p.surface1);
    let style = TextStyle {
        font: Font::Sans,
        weight: Weight::Bold,
        size: 16.0,
        color: p.peach,
    };
    frame.text("cache.get(route)", 12.0, 40.0, 24.0, style);
    frame.quad(Rect::new(150.0, 70.0, 60.0, 30.0), p.green);
    frame.layer();
    frame.glass(
        Rect::new(30.0, 24.0, 180.0, 72.0),
        (12.0, 14.0),
        p.text.with_alpha(30),
    );
    frame.text(
        "Save task",
        48.0,
        24.0,
        72.0,
        TextStyle {
            color: p.text,
            ..style
        },
    );
    compare("glass", size, &frame);
}

#[test]
fn a_veil_dims_the_layers_below_and_not_its_own() {
    let size = Size::new(16, 16);
    let mut r = renderer(size);
    let mut frame = Frame::new(size, Color::WHITE);
    frame.layer();
    frame.veil(Color::BLACK.with_alpha(128));
    frame.quad(Rect::new(8.0, 0.0, 8.0, 16.0), Color::WHITE);
    let image = r.snapshot(&frame).expect("snapshot");
    assert!(
        (100..200).contains(&image.pixel(2, 8)[0]),
        "{:?}",
        image.pixel(2, 8)
    );
    assert_eq!(image.pixel(12, 8)[0], 255);
}
