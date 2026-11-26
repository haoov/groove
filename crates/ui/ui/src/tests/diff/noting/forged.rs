//! What the forge answers for: a thread replied to or resolved.

use super::*;
use crate::tests::typed;
use groove_controllers::delivery;

/// The app with an MR of its own, and these threads on it.
fn delivered(notes: Vec<groove_types::Note>) -> AppState {
    let mut app = opened();
    app.delivery.shown = notes;
    app.delivery.remembered(vec![groove_types::Mr {
        id: groove_types::MrId::new("m1"),
        worktree: groove_types::WorktreeId::new("wt-1"),
        forge: groove_types::Forge::Gitlab,
        remote_id: "7".into(),
        url: "https://example.com/7".into(),
        state: groove_types::MrState::Open,
    }]);
    app
}

/// A click on one button of one thread's row.
fn on(
    app: &AppState,
    ui: &mut Ui,
    thread: &str,
    button: crate::hit::NoteButton,
) -> Vec<groove_controllers::Command> {
    let (_, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let rect = hits
        .rect_of(&Target::Note(thread.into(), button))
        .unwrap_or_else(|| panic!("{button:?} is drawn"));
    click(rect, ui, app, &hits)
}

#[test]
fn a_thread_offers_a_reply_and_a_resolve_and_nothing_else() {
    let app = delivered(vec![thread()]);
    let ui = on_diff();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    for button in [
        crate::hit::NoteButton::Reply,
        crate::hit::NoteButton::Resolve,
    ] {
        assert!(
            hits.rect_of(&Target::Note("t1".into(), button)).is_some(),
            "{button:?} stands under the thread"
        );
    }
}

#[test]
fn resolving_a_thread_asks_the_forge() {
    let app = delivered(vec![thread()]);
    let mut ui = on_diff();
    let commands = on(&app, &mut ui, "t1", crate::hit::NoteButton::Resolve);
    assert_eq!(
        one_command(commands),
        delivery::Command::Thread(groove_controllers::delivery::ThreadAct::Resolve {
            thread: "t1".into(),
            resolve: true
        })
    );
}

#[test]
fn a_resolved_thread_asks_to_be_opened_again() {
    let mut one = thread();
    one.resolved = true;
    let app = delivered(vec![one]);
    let mut ui = on_diff();
    on(&app, &mut ui, "t1", crate::hit::NoteButton::Fold);
    let commands = on(&app, &mut ui, "t1", crate::hit::NoteButton::Resolve);
    assert_eq!(
        one_command(commands),
        delivery::Command::Thread(groove_controllers::delivery::ThreadAct::Resolve {
            thread: "t1".into(),
            resolve: false
        })
    );
}

#[test]
fn a_reply_is_typed_under_the_thread_it_answers() {
    let app = delivered(vec![thread()]);
    let mut ui = on_diff();
    let commands = on(&app, &mut ui, "t1", crate::hit::NoteButton::Reply);
    assert!(commands.is_empty(), "nothing is asked of the forge yet");
    let noting = ui.session.noting.as_ref().expect("the row opens");
    assert_eq!(noting.anchor, Anchor::line("src/lib.rs", 1));
    assert!(noting.said().is_empty(), "with nothing in it");

    typed("fixed", &mut ui, &app);
    let commands = press(Key::Enter, &mut ui, &app);
    assert_eq!(
        one_command(commands),
        delivery::Command::Thread(groove_controllers::delivery::ThreadAct::Reply {
            thread: "t1".into(),
            body: "fixed".into()
        })
    );
}

#[test]
fn a_reply_is_typed_inside_the_thread_s_card_in_place_of_its_buttons() {
    let app = delivered(vec![thread()]);
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    let before = crate::views::session::diff::rows_of(&app, &ui, drawn_cols(&app, &ui));
    on(&app, &mut ui, "t1", crate::hit::NoteButton::Reply);
    typed("fixed", &mut ui, &app);
    assert_eq!(
        crate::views::session::diff::rows_of(&app, &ui, drawn_cols(&app, &ui)),
        before + 1,
        "one line of reply and its buttons where the buttons stood, and no card of its own"
    );
    let drawn = row_texts(&app, &ui);
    let at = |text: &str| drawn.iter().position(|one| one == text);
    let (words, reply) = (at("this leaks"), at("fixed"));
    assert!(
        words.is_some() && reply > words,
        "under the thread's words: {drawn:?}"
    );
    assert!(
        at("Reply\u{2026}").is_none() && at("resolve").is_none(),
        "{drawn:?}"
    );
}

#[test]
fn a_line_with_a_thread_still_opens_the_line_menu() {
    let app = delivered(vec![thread()]);
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    asked(&app, &mut ui, 1);
    assert_eq!(
        ui.menu().map(|menu| &menu.of),
        Some(&crate::Of::Line {
            path: "src/lib.rs".into(),
            lines: (1, 1),
        }),
        "a thread holds no line"
    );
}
