use groove_gfx::{CellSize, Color, Edges, Fonts, Frame, Rect, Size};
use groove_types::{ThemeName, Timestamp};

use super::{Boxes, Spec};
use crate::base::ctx::{App, Ctx, Metrics};
use crate::base::style::Role;
use crate::base::tokens::Tokens;
use crate::text::wrapped;

/// An app that only counts what it is told to hit.
struct Nothing(usize);

impl App for Nothing {
    type Target = ();

    fn hit(&mut self, _: Rect, _: ()) {
        self.0 += 1;
    }
}

fn with_ctx<R>(draw: impl FnOnce(&mut Ctx<'_, Nothing>) -> R) -> R {
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
    let mut ctx = Ctx::new(
        ThemeName::default(),
        metrics,
        Nothing(0),
        &mut frame,
        &mut fonts,
        None,
    );
    draw(&mut ctx)
}

const WORDS: &str = "issue: this leaks the handle every time the file is opened again \
                     and nothing ever closes it";

#[test]
fn a_column_stacks_its_children_inside_its_padding_with_the_gap_between() {
    with_ctx(|ctx| {
        let mut layout = Boxes::new();
        let first = layout.leaf(Spec::default().height(10.0));
        let second = layout.leaf(Spec::default().height(20.0));
        let spec = Spec::default().gap(4.0).pad(Edges::all(2.0));
        let root = layout.column(spec, &[first, second]);
        layout.solve(ctx, root, Rect::new(100.0, 50.0, 200.0, 300.0));
        assert_eq!(layout.rect(root), Rect::new(100.0, 50.0, 200.0, 300.0));
        assert_eq!(layout.rect(first), Rect::new(102.0, 52.0, 196.0, 10.0));
        assert_eq!(layout.rect(second), Rect::new(102.0, 66.0, 196.0, 20.0));
    });
}

#[test]
fn a_growing_leaf_takes_the_room_its_siblings_leave() {
    with_ctx(|ctx| {
        let mut layout = Boxes::new();
        let fixed = layout.leaf(Spec::default().width(30.0));
        let rest = layout.leaf(Spec::default().grow(1.0));
        let root = layout.row(Spec::default(), &[fixed, rest]);
        layout.solve(ctx, root, Rect::new(0.0, 0.0, 200.0, 40.0));
        assert_eq!(layout.rect(rest), Rect::new(30.0, 0.0, 170.0, 40.0));
    });
}

#[test]
fn a_text_leaf_wraps_to_the_width_it_is_given_in_the_same_solve() {
    with_ctx(|ctx| {
        let style = ctx.styles.body(Role::Text);
        let line = ctx.tokens.line;
        let tall = |ctx: &mut Ctx<'_, Nothing>, width: f32| {
            let mut layout = Boxes::new();
            let text = layout.text(Spec::default(), WORDS, style, line);
            let root = layout.column(Spec::default(), &[text]);
            layout.solve(ctx, root, Rect::new(0.0, 0.0, width, 800.0));
            let rect = layout.rect(text);
            assert_eq!(rect.w, width, "the leaf stretches across the column");
            let rows = wrapped(ctx, WORDS, &style, width).len();
            assert_eq!(
                rect.h,
                rows as f32 * line,
                "a row of `line` for each wrapped row"
            );
            rows
        };
        let narrow = tall(ctx, 120.0);
        let wide = tall(ctx, 1200.0);
        assert_eq!(wide, 1);
        assert!(narrow > 3, "{narrow} rows at 120px");
    });
}

#[test]
fn a_text_leaf_left_to_its_content_is_as_wide_as_its_longest_line() {
    with_ctx(|ctx| {
        let style = ctx.styles.body(Role::Text);
        let mut layout = Boxes::new();
        let text = layout.text(Spec::default(), "short\na longer line", style, 20.0);
        let root = layout.column(Spec::default().centred(), &[text]);
        layout.solve(ctx, root, Rect::new(0.0, 0.0, 600.0, 200.0));
        let rect = layout.rect(text);
        assert_eq!(rect.w, ctx.measure("a longer line", &style));
        assert_eq!(rect.h, 40.0);
    });
}

#[test]
fn a_solve_keeps_the_rows_it_wrapped_for_the_next_frame() {
    with_ctx(|ctx| {
        let style = ctx.styles.body(Role::Text);
        assert_eq!(ctx.kept_rows(WORDS, &style, 120.0), None);
        let mut layout = Boxes::new();
        let text = layout.text(Spec::default(), WORDS, style, 20.0);
        let root = layout.column(Spec::default(), &[text]);
        layout.solve(ctx, root, Rect::new(0.0, 0.0, 120.0, 800.0));
        let rows = wrapped(ctx, WORDS, &style, 120.0).len();
        assert_eq!(ctx.kept_rows(WORDS, &style, 120.0), Some(rows));
    });
}

/// Run with `cargo test --release -p groove-ui-kit time_ -- --ignored --nocapture`.
#[test]
#[ignore]
#[allow(clippy::print_stdout)]
fn time_a_solve_of_many_text_leaves() {
    use std::time::Instant;
    with_ctx(|ctx| {
        let style = ctx.styles.body(Role::Text);
        let line = ctx.tokens.line;
        let bodies: Vec<String> = (0..200).map(|at| format!("{at} {WORDS} {WORDS}")).collect();
        let solve = |ctx: &mut Ctx<'_, Nothing>, width: f32| {
            let mut layout = Boxes::new();
            let leaves: Vec<_> = bodies
                .iter()
                .map(|body| layout.text(Spec::default(), body, style, line))
                .collect();
            let root = layout.column(Spec::default().gap(4.0), &leaves);
            layout.solve(ctx, root, Rect::new(0.0, 0.0, width, 100_000.0));
        };
        let started = Instant::now();
        solve(ctx, 600.0);
        println!("cold: {:?}", started.elapsed());
        let runs = 20;
        let started = Instant::now();
        for _ in 0..runs {
            solve(ctx, 600.0);
        }
        println!("still: {:?} a solve", started.elapsed() / runs);
        let started = Instant::now();
        for at in 0..runs {
            solve(ctx, 400.0 + at as f32 * 10.0);
        }
        println!("resizing: {:?} a solve", started.elapsed() / runs);
    });
}

#[test]
fn a_box_with_a_set_size_keeps_it_when_the_room_runs_out() {
    let mut layout = Boxes::new();
    let fixed = layout.leaf(Spec::default().width(150.0));
    let rest = layout.leaf(Spec::default().grow(1.0));
    let root = layout.row(Spec::default(), &[fixed, rest]);
    layout.place(root, Rect::new(0.0, 0.0, 100.0, 40.0));
    assert_eq!(layout.rect(fixed).w, 150.0);
    assert_eq!(layout.rect(rest).w, 0.0);
}

#[test]
fn a_box_shrinks_below_what_its_children_hold() {
    let mut layout = Boxes::new();
    let fixed = layout.leaf(Spec::default().width(150.0));
    let inner = layout.row(Spec::default().grow(1.0), &[fixed]);
    let side = layout.leaf(Spec::default().width(30.0));
    let root = layout.row(Spec::default(), &[side, inner]);
    layout.place(root, Rect::new(0.0, 0.0, 100.0, 40.0));
    assert_eq!(layout.rect(inner).w, 70.0);
}

#[test]
#[ignore]
#[allow(clippy::print_stdout)]
fn time_building_and_placing_a_tree_of_boxes() {
    use std::time::Instant;
    let build = || {
        let mut layout = Boxes::new();
        let columns: Vec<_> = (0..4)
            .map(|_| {
                let parts: Vec<_> = (0..3)
                    .map(|_| layout.leaf(Spec::default().grow(1.0)))
                    .collect();
                layout.column(Spec::default().width(200.0), &parts)
            })
            .collect();
        let root = layout.row(Spec::default(), &columns);
        (layout, root)
    };
    let runs = 2000;
    let started = Instant::now();
    for _ in 0..runs {
        let _ = build();
    }
    let built = started.elapsed() / runs;
    let started = Instant::now();
    for _ in 0..runs {
        let (mut layout, root) = build();
        layout.place(root, Rect::new(0.0, 0.0, 1280.0, 800.0));
    }
    println!(
        "TIMED built {built:?}, built and placed {:?}",
        started.elapsed() / runs
    );
}
