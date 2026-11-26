//! The menu a right click opens on lines, and a reply typed in place under its thread.

mod asking;
mod composer;
mod forged;
mod typing;

use groove_controllers::delivery;
use groove_gfx::Fonts;
use groove_types::{Anchor, Caret, Selection};

use super::*;
use crate::input::{Key, Modifiers};
use crate::tests::typed;
use crate::views::session::Noting;

/// The right button on the row `line` of the open file.
fn asked(app: &AppState, ui: &mut Ui, line: usize) {
    let (_, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let rect = hits.rect_of(&Target::Code).expect("the rows");
    let y = rect.y + Tokens::new(1.0).code_line * line as f32 + 1.0;
    handle(
        Input::Menu {
            x: rect.x + 200.0,
            y,
        },
        ui,
        app,
        &hits,
        window(),
    );
}

/// A thread of the merge request, on line 1.
fn thread() -> groove_types::Note {
    groove_types::Note {
        thread: "t1".into(),
        anchor: Some(Anchor::line("src/lib.rs", 1)),
        resolved: false,
        said: vec![groove_types::Said {
            author: "reviewer".into(),
            body: "issue: this leaks".into(),
            at: groove_types::Timestamp::new(0),
        }],
    }
}

/// The thread on line 1, and a reply to it open in the file view with `text` typed in it.
fn replying(text: &str) -> (AppState, Ui) {
    let mut app = opened();
    app.delivery.shown = vec![thread()];
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    ui.session.noting = Some(Noting::reply(Anchor::line("src/lib.rs", 1), "t1".into()));
    ui.focus = crate::Focus::Workspace;
    typed(text, &mut ui, &app);
    (app, ui)
}

fn press(key: Key, ui: &mut Ui, app: &AppState) -> Vec<groove_controllers::Command> {
    handle(
        Input::Key {
            key,
            mods: Modifiers::default(),
        },
        ui,
        app,
        &crate::hit::Hits::default(),
        window(),
    )
}

#[test]
fn a_right_click_in_the_rows_asks_about_that_line() {
    let app = opened();
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    asked(&app, &mut ui, 1);
    assert_eq!(
        ui.menu().map(|menu| &menu.of),
        Some(&crate::Of::Line {
            path: "src/lib.rs".into(),
            lines: (1, 1),
        })
    );
}

#[test]
fn a_right_click_inside_a_selection_asks_about_all_of_it() {
    let mut app = opened();
    let file = app.workspace.active_mut().expect("the open file");
    file.new.holding(Selection {
        anchor: Caret::new(0, 0),
        head: Caret::new(2, 1),
    });
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    asked(&app, &mut ui, 1);
    assert_eq!(
        ui.menu().map(|menu| &menu.of),
        Some(&crate::Of::Line {
            path: "src/lib.rs".into(),
            lines: (0, 2),
        })
    );
}

#[test]
fn an_empty_reply_sends_nothing() {
    let (app, mut ui) = replying("");
    let commands = press(Key::Enter, &mut ui, &app);
    assert!(commands.is_empty(), "{commands:?}");
    assert!(ui.session.noting.is_none());
}

#[test]
fn escape_drops_the_words_and_asks_nothing() {
    let (app, mut ui) = replying("issue");
    let commands = press(Key::Escape, &mut ui, &app);
    assert!(commands.is_empty(), "{commands:?}");
    assert!(ui.session.noting.is_none());
}

#[test]
fn what_is_pasted_goes_into_the_reply() {
    let (app, mut ui) = replying("");
    handle(
        Input::Paste("issue: this leaks".into()),
        &mut ui,
        &app,
        &crate::hit::Hits::default(),
        window(),
    );
    assert_eq!(
        ui.session.noting.as_ref().map(|one| one.said().to_string()),
        Some("issue: this leaks".to_string())
    );
}

/// The one command a keystroke asked for.
fn one_command(commands: Vec<groove_controllers::Command>) -> delivery::Command {
    match commands.into_iter().next() {
        Some(groove_controllers::Command::Delivery(command)) => command,
        other => panic!("one delivery command, not {other:?}"),
    }
}

#[test]
fn hovering_one_threads_button_leaves_the_others_alone() {
    let mut app = opened();
    let mut second = thread();
    second.thread = "t2".into();
    second.anchor = Some(Anchor::line("src/lib.rs", 2));
    app.delivery.shown = vec![thread(), second];
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let mut ui = on_diff();
    ui.hover = Some(Target::Note("t1".into(), crate::hit::NoteButton::Resolve));
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let buttons: Vec<groove_gfx::Rect> = ["t1", "t2"]
        .into_iter()
        .filter_map(|id| hits.rect_of(&Target::Note(id.into(), crate::hit::NoteButton::Resolve)))
        .collect();
    let lit = frame
        .layers()
        .iter()
        .flat_map(|layer| layer.quads.iter())
        .filter(|quad| quad.color == styles.glass_hover() && buttons.contains(&quad.rect))
        .count();
    assert_eq!(buttons.len(), 2, "a resolve under each thread");
    assert_eq!(lit, 1, "one button of one thread");
}

#[test]
fn shift_enter_breaks_the_line_and_the_reply_keeps_it() {
    let (app, mut ui) = replying("fixed");
    let before = crate::views::session::diff::rows_of(&app, &ui, drawn_cols(&app, &ui));
    let shift = Modifiers {
        shift: true,
        ..Modifiers::default()
    };
    let input = Input::Key {
        key: Key::Enter,
        mods: shift,
    };
    handle(input, &mut ui, &app, &crate::hit::Hits::default(), window());
    typed("drop it", &mut ui, &app);
    assert_eq!(
        crate::views::session::diff::rows_of(&app, &ui, drawn_cols(&app, &ui)),
        before + 1,
        "a row per line"
    );
    assert_eq!(
        one_command(press(Key::Enter, &mut ui, &app)),
        delivery::Command::Thread(groove_controllers::delivery::ThreadAct::Reply {
            thread: "t1".into(),
            body: "fixed\ndrop it".into(),
        })
    );
}
