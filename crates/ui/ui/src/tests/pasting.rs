//! The clipboard: which field takes it, and what asks for it instead.

use groove_controllers::{Command, workspace};
use groove_gfx::Fonts;

use crate::input::{Input, Key, Modifiers, handle};
use crate::tests::{full_app, press, window};
use crate::views::session::{Tab, Term};
use crate::{Surface, Ui, view};

fn paste(text: &str, ui: &mut Ui, app: &groove_controllers::AppState) -> Vec<Command> {
    let (_, hits) = view(app, ui, window(), &mut Fonts::embedded());
    handle(Input::Paste(text.into()), ui, app, &hits, window())
}

const URL: &str = "git@gitlab.example.com:g/kiwi.git";

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
        ui.palette().map(|one| one.query.text()),
        Some(URL),
        "the url is in the query"
    );

    paste("/second", &mut ui, &app);
    assert_eq!(
        ui.palette().map(|one| one.query.text()),
        Some("git@gitlab.example.com:g/kiwi.git/second"),
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
        ui.palette().map(|one| one.query.text()),
        Some("one two"),
        "a field holds one line, its breaks spaces"
    );
}

#[test]
fn the_search_bar_and_the_board_filter_take_it_too() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.tab = Tab::Diff;
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
    ui.session.tab = Tab::Diff;
    ui.session.composing = true;
    ui.focus = crate::Focus::Sidebar;
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
    ui.session.tab = Tab::Diff;
    ui.session.composing = true;
    ui.focus = crate::Focus::Sidebar;
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
    let app = crate::tests::diff::opened();
    let mut ui = Ui {
        focus: crate::Focus::Workspace,
        ..Ui::default()
    };
    ui.session.tab = Tab::Diff;
    let asked = paste("one", &mut ui, &app);
    assert_eq!(
        asked,
        vec![Command::Workspace(workspace::Command::Edit(
            groove_types::Edit::Insert("one".into())
        ))],
        "the text read once goes in as it is"
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
    ui.session.tab = Tab::Diff;
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

#[test]
fn on_the_board_a_paste_reaches_no_hidden_session_field() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.resources.typing = true;
    ui.surface = Surface::Board;
    ui.settle(&app);
    let asked = paste("app=web", &mut ui, &app);
    assert!(asked.is_empty(), "no file takes it");
    assert_eq!(
        ui.session.resources.search.text(),
        "",
        "nor the hidden search"
    );
    assert!(!ui.session.resources.typing, "which let go of the keys");
}

#[test]
fn the_resources_search_takes_what_is_pasted_into_it() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.tab = Tab::Resources;
    ui.session.resources.typing = true;
    let asked = paste("app=web", &mut ui, &app);
    assert!(asked.is_empty());
    assert_eq!(ui.session.resources.search.text(), "app=web");
}

#[test]
fn a_field_on_a_tab_not_shown_takes_no_paste() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.tab = Tab::Overview;
    ui.session.bar.open(Term::Path);
    paste("src/one", &mut ui, &app);
    assert_eq!(ui.session.bar.path.text(), "", "the bar is not drawn");
}

#[test]
fn the_commit_box_takes_no_paste_once_another_pane_has_the_keys() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.tab = Tab::Diff;
    ui.session.composing = true;
    ui.focus = crate::Focus::Agent;
    let asked = paste("one", &mut ui, &app);
    assert!(
        matches!(asked.as_slice(), [Command::Agent(_)]),
        "the agent takes it: {asked:?}"
    );
}

#[test]
fn a_paste_in_the_text_term_searches_as_typing_does() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.tab = Tab::Diff;
    ui.session.bar.open(Term::Text);
    let asked = paste("needle", &mut ui, &app);
    assert_eq!(
        asked,
        vec![Command::Workspace(workspace::Command::Grep {
            query: "needle".into(),
            under: String::new(),
        })]
    );
}

#[test]
fn the_note_and_the_commit_box_take_a_paste_alike_without_its_carriage_returns() {
    let app = crate::tests::diff::opened();
    let mut ui = Ui::default();
    ui.session.tab = Tab::Diff;
    ui.session.composing = true;
    ui.focus = crate::Focus::Sidebar;
    let asked = paste("  one\r\n  two\r\n", &mut ui, &app);
    let kept = groove_types::Edit::Insert("  one\n  two\n".into());
    assert_eq!(
        asked,
        vec![Command::Workspace(workspace::Command::Message(kept))]
    );

    let anchor = groove_types::Anchor::line("src/lib.rs", 0);
    ui.session.noting = Some(crate::views::session::Noting::new(anchor));
    ui.focus = crate::Focus::Workspace;
    paste("  one\r\n  two\r\n", &mut ui, &app);
    let typed = ui.session.noting.as_ref().map(|one| one.buffer.text());
    assert_eq!(typed.as_deref(), Some("  one\n  two\n"));
}

#[test]
fn on_the_overview_the_hidden_open_file_takes_no_key_and_no_paste() {
    let app = crate::tests::diff::opened();
    let mut ui = Ui {
        focus: crate::Focus::Workspace,
        ..Ui::default()
    };
    ui.session.tab = Tab::Overview;
    assert!(paste("one", &mut ui, &app).is_empty(), "no paste edits it");
    let keys = [
        (Key::Char('a'), Modifiers::default()),
        (Key::Enter, Modifiers::default()),
        (Key::Tab, Modifiers::default()),
        (Key::Backspace, Modifiers::default()),
        (
            Key::Char('v'),
            Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        ),
    ];
    for (key, mods) in keys {
        let asked = press(key, mods, &mut ui, &app);
        assert!(asked.is_empty(), "{key:?} edits nothing: {asked:?}");
    }
}
