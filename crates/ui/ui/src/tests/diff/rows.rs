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
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
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
fn the_gutters_end_at_a_hairline_and_the_text_starts_past_it() {
    let app = opened();
    let ui = on_diff();
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let code = hits.rect_of(&Target::Code).expect("the rows are drawn");
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let rule = frame.layers()[0]
        .quads
        .iter()
        .find(|quad| quad.color == styles.line() && quad.rect.h == code.h && quad.rect.x > code.x)
        .expect("a hairline down the gutters");
    let numbers: Vec<f32> = frame.layers()[0]
        .texts
        .iter()
        .filter(|run| run.text.parse::<u32>().is_ok() && code.contains(run.x, run.y))
        .map(|run| run.x)
        .collect();
    assert!(!numbers.is_empty());
    assert!(
        numbers.iter().all(|x| *x < rule.rect.x),
        "the numbers sit left of it: {numbers:?} against {}",
        rule.rect.x
    );
    let text = frame.layers()[0]
        .texts
        .iter()
        .find(|run| run.text == "fn")
        .expect("a line of code");
    let gap = text.x - rule.rect.x;
    assert!(gap >= Tokens::new(1.0).md, "the text clears it: {gap}");
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
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let panels = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.panel() && quad.rect.h == Tokens::new(1.0).line)
        .count();
    assert_eq!(panels, 1, "one band, the height of a row");
}

#[test]
fn a_changed_row_shades_the_word_that_changed_and_not_the_rest() {
    let app = opened();
    let ui = on_diff();
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
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
    ui.session.view = DiffView::File;
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
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
