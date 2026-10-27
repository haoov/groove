use groove_gfx::Rect;

use super::{Cell, Column, Rows, Shown, Table, Width};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::base::tokens::Tokens;
use crate::tests::drawn::{drawn, drawn_at};

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
            carded: false,
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
    let barred = || Cell::small(sample, Role::Text).bar(Some(1.0), None, Role::Bad);
    let fit = barred();
    let columns = [column(Width::FitCell(&fit)), column(Width::Fill)];
    let table = Table {
        columns: &columns,
        rows: Rows {
            first: 24.0,
            under: 0.0,
            ruled: false,
            carded: false,
        },
        count: 1,
        offset: 0.0,
        selected: None,
        sorted: None,
    };
    let wide = Rect { w: 1000.0, ..ROOM };
    let (((x, w), bad, text), frame) = drawn(|ctx| {
        let span = table.spans(ctx, wide)[0];
        table.one(ctx, wide, Shown::row(vec![barred()]));
        let text = ctx.measure(sample, &ctx.styles.small(Role::Text));
        (span, ctx.styles.color(Role::Bad), text)
    });
    let layer = &frame.layers()[0];
    let mut bar = layer.quads.iter().filter(|one| one.color == bad);
    assert!(bar.clone().count() > 0, "the bar drawn");
    assert!(bar.all(|one| one.rect.x >= x && one.rect.right() <= x + w + 0.01));
    let drawn = layer
        .texts
        .iter()
        .find(|one| one.text == sample)
        .expect("whole");
    assert!(
        drawn.x >= x && drawn.x + text <= x + w + 0.01,
        "the text within"
    );
}

#[test]
fn a_cell_in_a_column_fitted_to_it_is_never_cut() {
    let samples = [
        "0.25 / 2",
        "minikube",
        "9999Mi / 999Gi",
        "WMWMWMWMWM",
        "iiiiiiiiijjj",
    ];
    let kinds: [fn(&str) -> Cell<'_>; 5] = [
        |text| Cell::small(text, Role::Text),
        |text| Cell::body(text, Role::Text),
        |text| Cell::label(text, Role::Text),
        |text| Cell::small(text, Role::Text).bar(Some(0.5), None, Role::Ok),
        |text| Cell::label(text, Role::Text).mark(Mark::Down, 0, Role::Text),
    ];
    for scale in [1.0, 1.1, 1.25, 1.5, 1.75, 2.0] {
        for sample in samples {
            for kind in kinds {
                let fit = kind(sample);
                let columns = [column(Width::FitCell(&fit)), column(Width::Fill)];
                let table = Table {
                    columns: &columns,
                    rows: Rows {
                        first: 24.0 * scale,
                        under: 0.0,
                        ruled: false,
                        carded: false,
                    },
                    count: 1,
                    offset: 0.0,
                    selected: None,
                    sorted: None,
                };
                let room = Rect::new(0.0, 0.0, 1000.0 * scale, 24.0 * scale);
                let row = Shown::row(vec![kind(sample)]);
                let (_, frame) = drawn_at(scale, |ctx| table.one(ctx, room, row));
                let texts = &frame.layers()[0].texts;
                let cut = texts.iter().find(|one| one.text.contains('…'));
                assert!(cut.is_none(), "{sample:?} at {scale}: {cut:?}");
            }
        }
    }
}

#[test]
fn a_selected_band_stands_on_a_rounded_held_ground_with_no_rules() {
    let columns = [column(Width::Fill)];
    let table = Table {
        columns: &columns,
        rows: Rows {
            first: 24.0,
            under: 0.0,
            ruled: false,
            carded: false,
        },
        count: 1,
        offset: 0.0,
        selected: Some(0),
        sorted: None,
    };
    let band = Shown::band(vec![Cell::small("pods", Role::Text)]);
    let ((held, chosen), frame) = drawn(|ctx| {
        table.one(ctx, ROOM, band);
        (ctx.styles.glass_chosen(), ctx.styles.chosen())
    });
    let quads = &frame.layers()[0].quads;
    let line = Rect { h: 24.0, ..ROOM };
    let ground = quads.iter().find(|one| one.color == held).expect("held");
    assert!(ground.rect.w < line.w, "set in from the sides");
    assert!(ground.shape.radius > 0.0, "and rounded");
    assert_eq!(quads.iter().filter(|one| one.color == chosen).count(), 0);
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
            carded: false,
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

#[test]
fn laid_stands_where_draw_puts_the_rows() {
    let columns = [column(Width::Fit("kind")), column(Width::Fill)];
    let table = Table {
        columns: &columns,
        rows: Rows {
            first: 24.0,
            under: 0.0,
            ruled: true,
            carded: false,
        },
        count: 1,
        offset: 0.0,
        selected: None,
        sorted: Some(super::Sorted::NONE),
    };
    let (laid, drawn) = drawn(|ctx| {
        let laid = table.laid(ctx, ROOM);
        (laid, table.one(ctx, ROOM, Shown::row(Vec::<Cell>::new())))
    })
    .0;
    assert_eq!(laid.spans, drawn.spans);
    assert_eq!(laid.body, drawn.body);
    let from = laid.from(0, 1);
    assert_eq!(from.x, laid.cell(0, 1).x, "from the column's left edge");
    assert_eq!(from.right(), ROOM.right(), "to the end");
}

#[test]
fn a_body_cell_in_a_column_fitted_to_its_sample_is_never_cut() {
    let samples = [
        "shop-worker-5d6b9c7f4",
        "waiting · CrashLoopBackOff",
        "WMWMWMWMWM",
    ];
    for scale in [1.0, 1.25, 1.5, 2.0] {
        for sample in samples {
            let columns = [column(Width::FitBody(sample)), column(Width::Fill)];
            let table = Table {
                columns: &columns,
                rows: Rows {
                    first: 26.0 * scale,
                    under: 0.0,
                    ruled: false,
                    carded: false,
                },
                count: 1,
                offset: 0.0,
                selected: None,
                sorted: None,
            };
            let room = Rect::new(0.0, 0.0, 1000.0 * scale, 26.0 * scale);
            let row = Shown::row(vec![Cell::body(sample, Role::Text)]);
            let (_, frame) = drawn_at(scale, |ctx| table.one(ctx, room, row));
            let texts = &frame.layers()[0].texts;
            let cut = texts.iter().find(|one| one.text.contains('…'));
            assert!(cut.is_none(), "{sample:?} at {scale}: {cut:?}");
        }
    }
}
