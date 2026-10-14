//! The tile a line carries in its marks' column for the notes it opens.

use super::*;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::{Role, Styles};

#[test]
fn a_line_counts_its_open_notes_in_a_tile_before_its_numbers() {
    let app = noted(vec![
        own(1, "issue: this leaks"),
        note(1, "reviewer", "nitpick: a name"),
    ]);
    let ui = on_diff();
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let code = hits.rect_of(&Target::Code).expect("the rows");
    let styles = Styles::new(app.config.theme(), Tokens::new(1.0));
    let layer = &frame.layers()[0];
    let tile = layer
        .quads
        .iter()
        .find(|quad| {
            quad.color == styles.color(Role::Warn) && code.contains(quad.rect.x, quad.rect.y)
        })
        .expect("a yellow tile")
        .rect;
    let count = layer
        .texts
        .iter()
        .find(|run| run.text == "2" && tile.contains(run.x + 1.0, run.y + 1.0))
        .expect("it counts both notes");
    let first_number = layer
        .texts
        .iter()
        .filter(|run| code.contains(run.x, run.y) && super::super::numeral(&run.text))
        .filter(|run| !tile.contains(run.x + 1.0, run.y + 1.0))
        .map(|run| run.x)
        .fold(f32::MAX, f32::min);
    assert!(
        tile.right() <= first_number,
        "before the numbers: {tile:?} {first_number}"
    );
    assert_eq!(count.style.color, styles.color(Role::Inverse));
    let note = Mark::Note.shape();
    assert!(
        !frame
            .layers()
            .iter()
            .flat_map(|one| one.icons.iter())
            .any(|icon| icon.icon == note && code.contains(icon.rect.x, icon.rect.y)),
        "the note's own row carries no mark of its own any more"
    );
}

#[test]
fn a_line_with_only_resolved_notes_ticks_and_a_click_opens_the_note() {
    let mut one = note(1, "reviewer", "issue: this leaks");
    one.resolved = true;
    let app = noted(vec![one]);
    let mut ui = on_diff();
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let code = hits.rect_of(&Target::Code).expect("the rows");
    let tick = Mark::Read.shape();
    let ticked = frame.layers()[0]
        .icons
        .iter()
        .find(|icon| icon.icon == tick && code.contains(icon.rect.x, icon.rect.y))
        .expect("a tick in the marks' column")
        .rect;
    let rows = |ui: &Ui| crate::views::session::diff::rows_of(&app, ui, drawn_cols(&app, ui));
    let folded = rows(&ui);
    let opens = Target::Note(
        NoteOrigin::Thread("t1".into()),
        crate::hit::NoteButton::Fold,
    );
    assert_eq!(hits.at(ticked.x + 2.0, ticked.y + 2.0), Some(opens));
    click(ticked, &mut ui, &app, &hits);
    assert!(rows(&ui) > folded, "the note opens out of its fold");
}
