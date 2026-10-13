use groove_gfx::{Edges, Rect};

use crate::tests::drawn::drawn;
use crate::widgets::Heading;

const LINE: Rect = Rect::new(100.0, 40.0, 300.0, 24.0);

fn said(heading: Heading<'_, ()>) -> String {
    let (_, frame) = drawn(|ctx| heading.draw(ctx, LINE));
    let texts = &frame.layers()[0].texts;
    texts
        .first()
        .map(|one| one.text.clone())
        .unwrap_or_default()
}

#[test]
fn a_heading_shows_its_count_else_its_note_else_its_name_alone() {
    assert_eq!(said(Heading::new("pods").count(3).note("idle")), "PODS · 3");
    assert_eq!(
        said(Heading::new("pods").count(0).note("idle")),
        "PODS · idle"
    );
    assert_eq!(said(Heading::new("pods").note("")), "PODS");
    assert_eq!(said(Heading::new("pods")), "PODS");
}

#[test]
fn a_heading_within_a_room_writes_its_name_there() {
    let room = LINE.pad(Edges::across(120.0, 0.0));
    let (_, frame) = drawn(|ctx| Heading::<()>::new("pods").within(ctx, LINE, room));
    let text = &frame.layers()[0].texts[0];
    assert!(
        text.x >= room.x,
        "{} starts in the room at {}",
        text.x,
        room.x
    );
}
