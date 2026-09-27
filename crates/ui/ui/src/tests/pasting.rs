//! The clipboard: which field takes it, and what asks for it instead.

use groove_controllers::{Command, workspace};
use groove_gfx::Fonts;

use crate::input::{Input, handle};
use crate::tests::{full_app, window};
use crate::views::session::{Tab, Term};
use crate::{Surface, Ui, view};

fn paste(text: &str, ui: &mut Ui, app: &groove_controllers::AppState) -> Vec<Command> {
    let (_, hits) = view(app, ui, window(), &mut Fonts::embedded());
    handle(Input::Paste(text.into()), ui, app, &hits, window())
}

const URL: &str = "git@gitlab.example.com:g/mayo.git";

#[test]
fn the_palette_takes_what_is_pasted_into_it() {
    let app = full_app();
    let mut ui = Ui {
        overlay: Some(crate::Overlay::Palette(crate::palette::Palette::default())),
        ..Ui::default()
    };
    let asked = paste(URL, &mut ui, &app);
    assert!(asked.is_empty(), "the field takes it on the spot");
    assert_eq!(
        ui.palette().map(|one| one.query.as_str()),
        Some(URL),
        "the url is in the query"
    );

    paste("/second", &mut ui, &app);
    assert_eq!(
        ui.palette().map(|one| one.query.as_str()),
        Some("git@gitlab.example.com:g/mayo.git/second"),
        "and a second paste follows the first"
    );
}

#[test]
fn a_pasted_line_break_never_reaches_a_field() {
    let app = full_app();
    let mut ui = Ui {
        overlay: Some(crate::Overlay::Palette(crate::palette::Palette::default())),
        ..Ui::default()
    };
    paste("one\ntwo\n", &mut ui, &app);
    assert_eq!(
        ui.palette().map(|one| one.query.as_str()),
        Some("one two"),
        "a field holds one line, its breaks spaces"
    );
}

#[test]
fn the_search_bar_and_the_board_filter_take_it_too() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.tab = Tab::File;
    ui.session.bar.open(Term::Path);
    paste("src/one", &mut ui, &app);
    assert_eq!(ui.session.bar.path.text(), "src/one");

    let mut board = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    board.board.focus();
    paste("priority:high", &mut board, &app);
    assert_eq!(board.board.filter.text(), "priority:high");
}

#[test]
fn the_commit_box_keeps_every_line_of_what_is_pasted() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.tab = Tab::File;
    ui.session.composing = true;
    let asked = paste("feat: one\n\nthe body", &mut ui, &app);
    assert_eq!(
        asked,
        vec![Command::Workspace(workspace::Command::Message(
            groove_types::Edit::Insert("feat: one\n\nthe body".into())
        ))],
        "a message is more than one line"
    );
}

#[test]
fn the_commit_box_takes_it_as_an_edit_of_its_own() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.tab = Tab::File;
    ui.session.composing = true;
    let asked = paste("feat: one", &mut ui, &app);
    assert_eq!(
        asked,
        vec![Command::Workspace(workspace::Command::Message(
            groove_types::Edit::Insert("feat: one".into())
        ))]
    );
}

#[test]
fn with_nothing_typing_the_open_file_takes_it() {
    let app = full_app();
    let mut ui = Ui {
        focus: crate::Focus::Workspace,
        ..Ui::default()
    };
    ui.session.tab = Tab::File;
    let asked = paste("one", &mut ui, &app);
    assert_eq!(
        asked,
        vec![Command::Workspace(workspace::Command::Paste)],
        "the buffer reads the clipboard itself"
    );
}

#[test]
fn an_empty_clipboard_asks_nothing() {
    let app = full_app();
    let mut ui = Ui::default();
    assert!(paste("\n", &mut ui, &app).is_empty());
    assert!(paste("", &mut ui, &app).is_empty());
}

#[test]
fn the_agent_takes_what_is_pasted_when_it_has_the_keyboard() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.tab = Tab::File;
    let asked = paste("one\ntwo", &mut ui, &app);
    assert_eq!(
        asked,
        vec![Command::Agent(groove_controllers::agent::Command::Paste {
            session: groove_types::SessionId::new("a"),
            text: "one\ntwo".into(),
        })],
        "the program types it, not the buffer"
    );
}
