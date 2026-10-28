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
    let numbers: Vec<&String> = drawn.iter().filter(|t| super::numeral(t)).collect();
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
            .filter(|t| t.starts_with(['+', '-']))
            .all(|t| super::numeral(&t[1..])),
        "no signs, only a head's counts: {drawn:?}"
    );
}

#[test]
fn a_changed_row_wears_one_tint_across_its_gutter_and_its_numbers_lean_to_it() {
    let app = opened();
    let ui = on_diff();
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let surface = crate::layout::Layout::of(window(), &ui).workspace;
    for kind in [groove_types::RowKind::Added, groove_types::RowKind::Removed] {
        let numbers = styles.gutter(kind).expect("a gutter");
        let row = styles.row_ground(kind).expect("a ground");
        let tints: Vec<_> = frame.layers()[0]
            .quads
            .iter()
            .filter(|quad| quad.color == row)
            .collect();
        assert!(!tints.is_empty(), "{kind:?} tinted");
        assert!(
            tints.iter().all(|quad| quad.rect.x <= surface.x + 1.0),
            "the tint starts under the gutter: {kind:?}"
        );
        assert!(
            frame.layers()[0]
                .texts
                .iter()
                .any(|text| text.style.color == numbers && super::numeral(&text.text)),
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
    let line = Tokens::new(1.0).code_line;
    let rules = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.line() && quad.rect.h == Tokens::new(1.0).hairline)
        .filter(|quad| quad.rect.y == at || quad.rect.bottom() == at + line)
        .count();
    assert_eq!(rules, 2, "the gap stands between two rules");
    let grounds = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.band() && quad.rect.h == line && quad.rect.y == at)
        .count();
    assert_eq!(grounds, 0, "on the rows' own ground, never a head's");
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

#[test]
fn a_gap_scrolled_half_out_keeps_its_arrows_inside_the_rows() {
    let old: String = (1..=60).map(|n| format!("line {n}\n")).collect();
    let new: String = (1..=60)
        .map(|n| match n % 20 == 1 {
            true => format!("LINE {n}\n"),
            false => format!("line {n}\n"),
        })
        .collect();
    let mut app = with_files();
    let paths: Vec<String> = (0..4).map(|n| format!("src/f{n}.rs")).collect();
    let files: Vec<(&str, &str, &str)> = paths
        .iter()
        .map(|path| (path.as_str(), old.as_str(), new.as_str()))
        .collect();
    crate::tests::changed_files(&mut app, &files);
    let (arrow, icon) = (
        groove_ui_kit::base::mark::Mark::Down.shape(),
        Tokens::new(1.0).icon,
    );
    let line = Tokens::new(1.0).code_line;
    let mut ui = on_diff();
    let mut above = Vec::new();
    for half in 0..80 {
        *ui.session.scroll_mut() = half as f32 * line / 2.0;
        let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
        let code = hits.rect_of(&crate::hit::Target::Code).expect("the rows");
        let gutter = code.x..code.x + line * 4.0;
        above.extend(
            frame
                .layers()
                .iter()
                .flat_map(|layer| layer.icons.iter())
                .filter(|one| one.icon == arrow && one.rect.w == icon)
                .filter(|one| gutter.contains(&one.rect.x))
                .map(|one| one.rect.intersect(one.clip))
                .filter(|seen| !seen.is_empty() && seen.y < code.y),
        );
    }
    assert!(
        above.is_empty(),
        "no arrow stands over the header: {above:?}"
    );
}
