//! The input method: what it composes stands at the caret, and what it types goes where a key would.

use groove_controllers::{AppState, Command, workspace};
use groove_gfx::Fonts;
use groove_types::Edit;

use crate::input::{Input, handle};
use crate::tests::diff::opened;
use crate::tests::{full_app, window};
use crate::views::session::Tab;
use crate::{Focus, Overlay, Surface, Ui, view};

fn filtering() -> Ui {
    let mut ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    ui.board.focus();
    ui
}

fn editing() -> Ui {
    let mut ui = Ui::default();
    ui.session.tab = Tab::Diff;
    ui.focus = Focus::Workspace;
    ui
}

fn sent(input: Input, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let (_, hits) = view(app, ui, window(), &mut Fonts::embedded());
    handle(input, ui, app, &hits, window())
}

#[test]
fn a_commit_types_each_character_into_the_field_that_has_the_keyboard() {
    let app = full_app();
    let mut ui = filtering();
    sent(Input::Commit("日本".into()), &mut ui, &app);
    assert_eq!(ui.board.filter.text(), "日本");
}

#[test]
fn a_commit_types_into_the_open_buffer_as_a_key_does() {
    let app = opened();
    let mut ui = editing();
    let asked = sent(Input::Commit("ê".into()), &mut ui, &app);
    assert_eq!(
        asked,
        vec![Command::Workspace(workspace::Command::Edit(Edit::Insert(
            "ê".into()
        )))]
    );
}

#[test]
fn a_commit_types_into_the_palette() {
    let app = full_app();
    let mut ui = Ui {
        overlay: Some(Overlay::Palette(crate::palette::Palette::default())),
        ..Ui::default()
    };
    sent(Input::Commit("ñ".into()), &mut ui, &app);
    let Some(Overlay::Palette(palette)) = &ui.overlay else {
        panic!("the palette closed");
    };
    assert_eq!(palette.query, "ñ");
}

#[test]
fn a_commit_lets_the_preedit_go() {
    let app = full_app();
    let mut ui = filtering();
    sent(Input::Preedit(Some("ni".into())), &mut ui, &app);
    assert_eq!(ui.preedit.as_deref(), Some("ni"));
    sent(Input::Commit("你".into()), &mut ui, &app);
    assert_eq!(ui.preedit, None);
}

#[test]
fn the_preedit_stands_before_the_caret_of_the_field_that_has_the_keyboard() {
    let app = full_app();
    let mut ui = filtering();
    ui.board.filter.set("ab");
    ui.preedit = Some("ni".into());
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let preedit = frame
        .layers()
        .iter()
        .flat_map(|layer| layer.texts.iter())
        .find(|one| one.text == "ni")
        .expect("the preedit is drawn");
    let caret = hits.caret().expect("the caret is reported");
    assert!(caret.x > preedit.x, "{caret:?} after {}", preedit.x);
}

#[test]
fn the_open_buffer_reports_its_caret_for_the_input_method_window() {
    let app = opened();
    let ui = editing();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(hits.caret().is_some());
}

#[test]
fn no_caret_is_reported_while_nothing_takes_typing() {
    let app = full_app();
    let ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    assert_eq!(hits.caret(), None);
}
