//! A resolved note folded to one line, and opened out of it.

use super::*;

#[test]
fn a_resolved_note_folds_to_one_line_and_a_click_opens_it_and_folds_it_back() {
    let mut one = note(1, "reviewer", "issue (blocking): this leaks");
    one.said.push(said("haoov", "fixed"));
    one.resolved = true;
    let (bare, app) = (opened(), noted(vec![one]));
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    let rows = |ui: &Ui, app: &AppState| {
        crate::views::session::diff::rows_of(app, ui, drawn_cols(app, ui))
    };
    assert_eq!(
        rows(&ui, &app),
        rows(&ui, &bare) + 3,
        "one line, room above and below"
    );
    let drawn = row_texts(&app, &ui);
    for text in ["issue", "this leaks", "reviewer", "1 reply"] {
        assert!(drawn.iter().any(|one| one == text), "{text}: {drawn:?}");
    }
    assert!(
        !drawn.iter().any(|one| one == "fixed"),
        "the reply stays folded"
    );
    let fold = Target::Note(
        NoteOrigin::Thread("t1".into()),
        crate::hit::NoteButton::Fold,
    );
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let line = hits
        .rect_of(&fold)
        .expect("the folded line opens on a click");
    click(line, &mut ui, &app, &hits);
    assert!(rows(&ui, &app) > rows(&ui, &bare) + 3, "opened out");
    assert!(row_texts(&app, &ui).iter().any(|one| one == "reopen"));
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let header = hits.rect_of(&fold).expect("its header folds it back");
    click(header, &mut ui, &app, &hits);
    assert_eq!(rows(&ui, &app), rows(&ui, &bare) + 3);
}
