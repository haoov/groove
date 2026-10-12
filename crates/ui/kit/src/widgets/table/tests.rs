use groove_gfx::{CellSize, Color, Fonts, Frame, Rect, Size};
use groove_types::{ThemeName, Timestamp};

use super::{Cell, Column, Rows, Shown, Table, Width};
use crate::base::ctx::{App, Ctx, Metrics};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::base::tokens::Tokens;

struct Nothing;

impl App for Nothing {
    type Target = ();

    fn hit(&mut self, _: Rect, _: ()) {}
}

/// Runs `draw` on a fresh frame; returns what it returned and the frame.
fn drawn<R>(draw: impl FnOnce(&mut Ctx<'_, Nothing>) -> R) -> (R, Frame) {
    let design = Tokens::new(1.0);
    let metrics = Metrics {
        size: Size::new(1280, 800),
        scale: 1.0,
        text: design.text,
        code: design.code,
        terminal: design.code,
        cell: CellSize {
            width: 8.0,
            height: 16.0,
        },
        advance: 8.0,
        tick: 0,
        now: Timestamp::new(0),
    };
    let mut frame = Frame::new(metrics.size, Color::BLACK);
    let mut fonts = Fonts::embedded();
    let theme = ThemeName::default();
    let out = {
        let mut ctx = Ctx::new(theme, metrics, Nothing, &mut frame, &mut fonts, None);
        draw(&mut ctx)
    };
    (out, frame)
}

const ROOM: Rect = Rect::new(100.0, 40.0, 300.0, 24.0);

fn column(width: Width<'_>) -> Column<'_, ()> {
    Column {
        label: "",
        width,
        end: false,
        sort: None,
        edge: None,
    }
}

/// The width of a column before a fill, in a room wide enough for both.
fn fitted(width: Width<'_>) -> f32 {
    let columns = [column(width), column(Width::Fill)];
    let table = Table {
        columns: &columns,
        rows: Rows {
            first: 24.0,
            under: 0.0,
            ruled: false,
        },
        count: 0,
        offset: 0.0,
        selected: None,
        sorted: None,
    };
    let wide = Rect { w: 1000.0, ..ROOM };
    drawn(|ctx| table.spans(ctx, wide)[0].1).0
}

#[test]
fn a_bar_with_no_text_still_draws() {
    let (_, frame) = drawn(|ctx| {
        let cell = Cell::small("", Role::Text).bar(Some(0.5), None, Role::Bad);
        cell.draw(ctx, ROOM, false)
    });
    let bad = drawn(|ctx| ctx.styles.color(Role::Bad)).0;
    let quads = &frame.layers()[0].quads;
    assert!(quads.iter().any(|quad| quad.color == bad), "the bar's fill");
}

#[test]
fn a_badge_stands_against_the_end() {
    let ((from, to), _) = drawn(|ctx| Cell::small("hook", Role::Ok).badge().draw(ctx, ROOM, true));
    assert!((to - ROOM.right()).abs() < 1.0, "ends at {to}");
    assert!(from > ROOM.x, "starts at {from}");
}

#[test]
fn a_lone_mark_takes_the_label_size_and_the_end() {
    let ((from, to), _) = drawn(|ctx| {
        let cell = Cell::label("", Role::Text).mark(Mark::Down, 0, Role::Text);
        cell.draw(ctx, ROOM, true)
    });
    let icon = Tokens::new(1.0).icon;
    assert!((to - from - icon).abs() < 0.5, "as wide as an icon");
    assert!((to - ROOM.right()).abs() < 0.5, "against the end");
}

#[test]
fn a_fitted_cell_counts_its_size_and_its_bar() {
    let sample = "9999Mi / 999Gi";
    let label = Cell::label(sample, Role::Text);
    let (wide, small) = drawn(|ctx| {
        let small = ctx.measure(sample, &ctx.styles.small(Role::Text));
        (label.wide(ctx), small)
    })
    .0;
    assert!(wide > small, "the label size is wider");
    assert!(
        fitted(Width::FitCell(&label)) >= wide,
        "the label text fits"
    );
    let barred = Cell::small(sample, Role::Text).bar(None, None, Role::Text);
    let tokens = Tokens::new(1.0);
    let meter = tokens.aside_near / 2.0 + tokens.sm + small;
    assert_eq!(fitted(Width::FitCell(&barred)), meter, "the meter's width");
}

#[test]
fn a_folding_band_leads_with_the_kit_caret() {
    let columns = [column(Width::Fill)];
    let table = Table {
        columns: &columns,
        rows: Rows {
            first: 24.0,
            under: 0.0,
            ruled: false,
        },
        count: 1,
        offset: 0.0,
        selected: None,
        sorted: None,
    };
    let band = |open| Shown::band(vec![Cell::small("pods", Role::Text)]).fold(open);
    let (_, frame) = drawn(|ctx| table.one(ctx, ROOM, band(false)));
    let layer = &frame.layers()[0];
    let [caret] = layer.icons.as_slice() else {
        panic!("one caret: {:?}", layer.icons.len());
    };
    assert_eq!(caret.turn, Mark::RIGHTWARDS, "shut points right");
    let text = layer
        .texts
        .iter()
        .find(|one| one.text == "pods")
        .expect("its name");
    assert!(text.x >= caret.rect.right(), "the name after the caret");
    let (_, plain) = drawn(|ctx| {
        let shown = Shown::band(vec![Cell::small("pods", Role::Text)]);
        table.one(ctx, ROOM, shown)
    });
    assert!(
        plain.layers()[0].icons.is_empty(),
        "no caret unless it folds"
    );
}
