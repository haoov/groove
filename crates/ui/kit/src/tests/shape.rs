//! What a list marks as chosen: the held ground, and the rules above and below.

use groove_gfx::Rect;

use crate::shape::{chosen_ground, chosen_rules};
use crate::tests::drawn::drawn;

const ROW: Rect = Rect::new(100.0, 40.0, 300.0, 24.0);

#[test]
fn the_chosen_ground_fills_the_row_on_the_held_ground() {
    let (held, frame) = drawn(|ctx| {
        chosen_ground(ctx, ROW);
        ctx.styles.held()
    });
    let quads = &frame.layers()[0].quads;
    assert_eq!(quads.len(), 1);
    assert_eq!((quads[0].rect, quads[0].color), (ROW, held));
}

#[test]
fn the_chosen_rules_run_along_the_row_s_top_and_bottom() {
    let ((color, thin), frame) = drawn(|ctx| {
        chosen_rules(ctx, ROW);
        (ctx.styles.chosen(), ctx.tokens.hairline)
    });
    let quads = &frame.layers()[0].quads;
    let rects: Vec<Rect> = quads.iter().map(|one| one.rect).collect();
    let top = Rect::new(ROW.x, ROW.y, ROW.w, thin);
    let bottom = Rect::new(ROW.x, ROW.bottom() - thin, ROW.w, thin);
    assert_eq!(rects, [top, bottom]);
    assert!(quads.iter().all(|one| one.color == color));
}
