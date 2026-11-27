//! The clipboard: which field takes it, and what asks for it instead.

use groove_controllers::{Command, workspace};
use groove_gfx::Fonts;

use crate::input::{Input, Key, Modifiers, handle};
use crate::tests::{full_app, press, window};
use crate::views::session::Mode;
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
fn with_nothing_typing_the_open_file_takes_it() {
    let app = crate::tests::diff::opened();
    let mut ui = Ui {
        focus: crate::Focus::Workspace,
        ..Ui::default()
    };
    ui.session.mode = Mode::Diff;
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
    ui.session.mode = Mode::Diff;
    let asked = paste("one\ntwo", &mut ui, &app);
    assert!(asked.is_empty(), "nothing is sent yet");
    assert_eq!(
        ui.agent.prompt.text(),
        "one\ntwo",
        "the prompt takes it, not the buffer"
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
    ui.session.mode = Mode::Kubernetes;
    ui.session.resources.typing = true;
    let asked = paste("app=web", &mut ui, &app);
    assert!(asked.is_empty());
    assert_eq!(ui.session.resources.search.text(), "app=web");
}

#[test]
fn a_field_on_a_mode_not_shown_takes_no_paste() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.mode = Mode::Overview;
    ui.session.resources.typing = true;
    paste("app=web", &mut ui, &app);
    assert_eq!(
        ui.session.resources.search.text(),
        "",
        "the list's search is not drawn"
    );
}

#[test]
fn the_reply_takes_a_paste_without_its_carriage_returns() {
    let mut app = crate::tests::diff::opened();
    let mut ui = Ui::default();
    ui.session.mode = Mode::Diff;
    let anchor = groove_types::Anchor::line("src/lib.rs", 0);
    app.delivery.shown = vec![crate::tests::diff::thread_on(&anchor)];
    ui.session.noting = Some(crate::views::session::Noting::reply(anchor, "t1".into()));
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
    ui.session.mode = Mode::Overview;
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
