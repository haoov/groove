//! A resolved note hidden behind its tick, and opened by it.

use super::*;

#[test]
fn a_resolved_note_takes_no_row_and_its_tick_opens_and_closes_it() {
    let mut one = note(1, "reviewer", "issue (blocking): this leaks");
    one.said.push(said("haoov", "fixed"));
    one.resolved = true;
    let (bare, app) = (opened(), noted(vec![one]));
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    let rows = |ui: &Ui, app: &AppState| {
        crate::views::session::diff::rows_of(app, ui, drawn_cols(app, ui))
    };
    assert_eq!(rows(&ui, &app), rows(&ui, &bare), "hidden behind its tick");
    assert!(!row_texts(&app, &ui).iter().any(|one| one == "this leaks"));
    let tick = Target::Note(
        NoteOrigin::Thread("t1".into()),
        crate::hit::NoteButton::Fold,
    );
    let toggle = |ui: &mut Ui| {
        let (_, hits) = view(&app, ui, window(), &mut Fonts::embedded());
        let at = hits.rect_of(&tick).expect("the tick takes a click");
        click(at, ui, &app, &hits);
    };
    toggle(&mut ui);
    assert!(rows(&ui, &app) > rows(&ui, &bare), "opened");
    let drawn = row_texts(&app, &ui);
    assert!(drawn.iter().any(|one| one == "this leaks"), "{drawn:?}");
    assert!(drawn.iter().any(|one| one == "reopen"), "{drawn:?}");
    toggle(&mut ui);
    assert_eq!(rows(&ui, &app), rows(&ui, &bare), "and hidden again");
}
