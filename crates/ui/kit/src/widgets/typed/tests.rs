use groove_gfx::{Color, Rect};

use crate::base::style::Role;
use crate::tests::drawn::drawn;
use crate::widgets::{caret_bar, text_band, typed_line};

const ROOM: Rect = Rect::new(100.0, 40.0, 300.0, 24.0);

#[test]
fn a_caret_is_a_bar_at_its_place_as_tall_as_its_span() {
    let span = Rect::new(100.0, 44.0, 50.0, 16.0);
    let (width, frame) = drawn(|ctx| {
        caret_bar(ctx, 120.0, span, Color::WHITE);
        ctx.tokens.hairline * 2.0
    });
    let quad = frame.layers()[0].quads[0];
    assert_eq!(quad.rect, Rect::new(120.0, 44.0, width, 16.0));
}

#[test]
fn a_caret_past_its_span_stands_at_the_span_s_end() {
    let span = Rect::new(100.0, 40.0, 50.0, 16.0);
    let (width, frame) = drawn(|ctx| {
        caret_bar(ctx, 400.0, span, Color::WHITE);
        ctx.tokens.hairline * 2.0
    });
    let quad = frame.layers()[0].quads[0];
    assert_eq!(quad.rect.right(), span.right());
    assert_eq!(quad.rect.w, width);
}

#[test]
fn a_band_runs_from_its_start_to_its_end_and_never_vanishes() {
    let (_, frame) = drawn(|ctx| {
        text_band(ctx, ROOM, (10.0, 50.0), Color::WHITE);
        text_band(ctx, ROOM, (20.0, 20.0), Color::WHITE);
    });
    let quads = &frame.layers()[0].quads;
    assert_eq!(quads[0].rect, Rect::new(110.0, 40.0, 40.0, 24.0));
    assert_eq!(quads[1].rect.w, 1.0, "an empty span still shows");
}

#[test]
fn a_selection_carried_past_a_line_s_end_adds_a_space_of_the_line_s_own_font() {
    let ((held, wide), frame) = drawn(|ctx| {
        let style = ctx.styles.body(Role::Text);
        typed_line(ctx, ROOM, ("ab", style), (None, Some((0, 2, true))));
        (
            ctx.styles.held(),
            ctx.measure("ab", &style) + ctx.measure(" ", &style),
        )
    });
    let quads = &frame.layers()[0].quads;
    let band = quads
        .iter()
        .find(|quad| quad.color == held)
        .expect("a band");
    assert!(
        (band.rect.w - wide).abs() < 0.01,
        "{} against {wide}",
        band.rect.w
    );
}
