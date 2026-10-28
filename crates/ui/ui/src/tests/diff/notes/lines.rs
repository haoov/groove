//! A note's lines: the range it names, its wrap, and the lines it keeps.

use super::*;

#[test]
fn a_note_says_which_lines_it_is_about() {
    let mut one = own(1, "issue: this leaks");
    one.anchor = Some(Anchor {
        path: "src/lib.rs".into(),
        start_line: 0,
        end_line: 2,
    });
    let app = noted(vec![one]);
    let drawn = in_editor(&app);
    assert!(
        drawn.iter().any(|text| text == "1-3"),
        "the lines a file numbers them: {drawn:?}"
    );
}

#[test]
fn a_note_on_one_line_says_that_line() {
    let app = noted(vec![own(1, "issue: this leaks")]);
    let drawn = in_editor(&app);
    assert!(drawn.iter().any(|text| text == "2"), "{drawn:?}");
}

#[test]
fn only_the_row_that_opens_a_note_says_its_lines() {
    let mut one = note(1, "reviewer", "issue: this leaks");
    one.said.push(said("haoov", "fixed"));
    let app = noted(vec![one]);
    let drawn = in_editor(&app);
    assert_eq!(
        drawn.iter().filter(|text| *text == "2").count(),
        2,
        "the line's own number, and the note's once: {drawn:?}"
    );
}

#[test]
fn a_long_note_wraps_onto_rows_of_its_own_and_loses_no_word() {
    let body = "issue: this leaks the handle every time the file is opened again \
                and nothing ever closes it";
    let app = noted(vec![note(1, "reviewer", body)]);
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    let drawn = row_texts(&app, &ui);
    let words = groove_types::labelled(body).expect("a label").rest;
    assert!(
        drawn.iter().any(|one| one == "issue"),
        "the label: {drawn:?}"
    );
    for word in words.split_whitespace() {
        assert!(
            drawn
                .iter()
                .any(|one| one.split_whitespace().any(|w| w == word)),
            "{word} is drawn: {drawn:?}"
        );
    }
    assert!(
        !drawn
            .iter()
            .any(|one| one.ends_with('\u{2026}') && one != "Reply\u{2026}"),
        "nothing is cut: {drawn:?}"
    );
    let rows = drawn
        .iter()
        .filter(|one| body.contains(one.trim()) && !one.trim().is_empty())
        .count();
    assert!(rows >= 4, "it takes the rows it needs: {drawn:?}");
}

#[test]
fn a_note_wraps_to_the_width_of_the_frame_that_draws_it() {
    let body = "issue: this leaks the handle every time the file is opened again \
                and nothing ever closes it, so a long session runs out of handles";
    let app = noted(vec![note(1, "reviewer", body)]);
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    let drawn_at = |width: u32| {
        let metrics = crate::tests::metrics(width, WINDOW.1, 1.0);
        let (frame, hits) = view(&app, &ui, metrics, &mut Fonts::embedded());
        let code = hits.rect_of(&Target::Code).expect("the rows are drawn");
        let rows = frame
            .layers()
            .iter()
            .flat_map(|layer| layer.texts.iter())
            .filter(|run| code.contains(run.x, run.y))
            .filter(|run| !run.text.trim().is_empty() && body.contains(run.text.trim()))
            .count();
        (hits.wrap(), rows)
    };
    let (narrow_cols, narrow) = drawn_at(WINDOW.0);
    let (wide_cols, wide) = drawn_at(WINDOW.0 * 3 / 2);
    assert!(wide_cols > narrow_cols, "{wide_cols} against {narrow_cols}");
    assert!(
        wide < narrow,
        "the first frame at a width wraps to it: {wide} rows against {narrow}"
    );
}

#[test]
fn a_note_keeps_the_lines_it_was_written_with() {
    let app = noted(vec![note(1, "reviewer", "first line\nsecond line")]);
    let drawn = in_editor(&app);
    assert!(drawn.iter().any(|one| one == "first line"), "{drawn:?}");
    assert!(drawn.iter().any(|one| one == "second line"), "{drawn:?}");
}
