//! What a row of the open file draws.

use super::*;

#[test]
fn the_tab_draws_the_open_file_with_both_its_numbers() {
    let app = opened();
    let drawn = row_texts(&app, &on_diff());
    assert!(drawn.iter().any(|t| t == "two"), "the old side: {drawn:?}");
    assert!(drawn.iter().any(|t| t == "TWO"), "the new side");
    assert!(
        drawn.iter().filter(|t| *t == "fn").count() == 4,
        "a keyword per row, coloured on its own: {drawn:?}"
    );
    let numbers: Vec<&String> = drawn.iter().filter(|t| t.parse::<u32>().is_ok()).collect();
    assert_eq!(
        numbers,
        ["1", "2", "2", "3"],
        "one number a row, the old one only where a line went: {drawn:?}"
    );
}

#[test]
fn a_row_says_what_it_is_with_its_ground_and_no_sign() {
    let app = opened();
    let ui = on_diff();
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let ground = |kind| {
        let color = styles.row_ground(kind).expect("a ground");
        frame.layers()[0]
            .quads
            .iter()
            .filter(|quad| quad.color == color)
            .count()
    };
    assert_eq!(ground(groove_types::RowKind::Added), 1);
    assert_eq!(ground(groove_types::RowKind::Removed), 1);
    let drawn = row_texts(&app, &ui);
    assert!(
        drawn
            .iter()
            .all(|t| !t.starts_with('+') && !t.starts_with('-')),
        "no signs: {drawn:?}"
    );
}

#[test]
fn a_changed_row_tints_its_gutter_and_its_numbers() {
    let app = opened();
    let ui = on_diff();
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    for kind in [groove_types::RowKind::Added, groove_types::RowKind::Removed] {
        let (ground, numbers) = styles.gutter(kind).expect("a gutter");
        let row = styles.row_ground(kind).expect("a ground");
        let width = |color| {
            frame.layers()[0]
                .quads
                .iter()
                .find(|quad| quad.color == color)
                .map(|quad| quad.rect.w)
                .expect("drawn")
        };
        assert!(
            width(ground) < width(row) / 4.0,
            "the gutter only: {kind:?}"
        );
        assert!(
            frame.layers()[0]
                .texts
                .iter()
                .any(|text| text.style.color == numbers && text.text.parse::<u32>().is_ok()),
            "its numbers lean to its colour: {kind:?}"
        );
    }
}

#[test]
fn a_comment_on_a_changed_row_stands_off_its_tint() {
    let mut app = with_files();
    crate::tests::shows(&mut app, "src/lib.rs", "// old words\n", "// new words\n");
    let ui = on_diff();
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let lifted = styles.changed_syntax(groove_types::Capture::Comment);
    assert_ne!(lifted, styles.syntax(groove_types::Capture::Comment));
    let drawn: Vec<&str> = frame.layers()[0]
        .texts
        .iter()
        .filter(|text| text.style.color == lifted)
        .map(|text| text.text.as_str())
        .collect();
    assert!(drawn.iter().any(|t| t.contains("old")), "{drawn:?}");
    assert!(drawn.iter().any(|t| t.contains("new")), "{drawn:?}");
}

#[test]
fn a_gap_reads_as_a_band_across_the_rows() {
    let old: String = (1..=40).map(|n| format!("line {n}\n")).collect();
    let new = old
        .replace("line 1\n", "LINE 1\n")
        .replace("line 40\n", "LINE 40\n");
    let mut app = with_files();
    crate::tests::shows(&mut app, "src/lib.rs", &old, &new);
    let ui = on_diff();
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let drawn = texts(&app, &ui);
    let band = drawn
        .iter()
        .find(|text| text.contains("lines"))
        .expect("the gap says how much it hides");
    assert!(band.starts_with('\u{2026}'), "{band}");
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let at = frame.layers()[0]
        .texts
        .iter()
        .find(|text| text.text.contains("lines"))
        .expect("the gap's own text")
        .y;
    let under = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.band() && quad.rect.h == Tokens::new(1.0).line)
        .filter(|quad| quad.rect.y == at)
        .count();
    assert_eq!(under, 1, "the gap stands on a band of its own");
}

#[test]
fn a_changed_row_shades_the_word_that_changed_and_not_the_rest() {
    let app = opened();
    let ui = on_diff();
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let shaded = |kind| {
        let color = styles.word(kind, None).expect("a word colour");
        frame.layers()[0]
            .quads
            .iter()
            .filter(|quad| quad.color == color)
            .map(|quad| quad.rect.w)
            .collect::<Vec<f32>>()
    };
    let added = shaded(groove_types::RowKind::Added);
    let removed = shaded(groove_types::RowKind::Removed);
    assert_eq!(added.len(), 1, "one word on the new side: {added:?}");
    assert_eq!(removed.len(), 1, "and one on the old");
    let row = crate::layout::Layout::of(window(), &ui).workspace.w;
    assert!(added[0] < row / 4.0, "the word only: {added:?}");
}

#[test]
fn the_file_view_shades_what_changed_inside_a_changed_line() {
    let app = opened();
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let color = styles
        .word(groove_types::RowKind::Context, Some(LineMark::Changed))
        .expect("a word colour");
    let shaded = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == color)
        .count();
    assert_eq!(shaded, 1, "the one word the change touched");
}
