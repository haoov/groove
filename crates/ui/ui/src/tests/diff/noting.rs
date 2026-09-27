//! Leaving a note: the menu a right click opens, the words typed in place, the note
//! the keyboard asks for.

mod forged;

use groove_controllers::delivery;
use groove_gfx::Fonts;
use groove_types::{Anchor, Caret, Selection};

use super::*;
use crate::input::{Key, Modifiers};
use crate::views::session::Noting;

/// The right button on the row `line` of the open file.
fn asked(app: &AppState, ui: &mut Ui, line: usize) {
    let (_, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let rect = hits.rect_of(&Target::Code).expect("the rows");
    let y = rect.y + Tokens::new(1.0).line * line as f32 + 1.0;
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

/// What picking the menu's first row leaves.
fn pick(app: &AppState, ui: &mut Ui) -> Vec<groove_controllers::Command> {
    let (_, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let row = hits.rect_of(&Target::MenuRow(0)).expect("a menu row");
    click(row, ui, app, &hits)
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

fn typed(text: &str, ui: &mut Ui, app: &AppState) {
    for c in text.chars() {
        press(Key::Char(c), ui, app);
    }
}

#[test]
fn a_right_click_in_the_rows_offers_a_note_on_that_line() {
    let app = opened();
    let mut ui = on_diff();
    ui.session.view = DiffView::Editor;
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
fn a_right_click_inside_a_selection_offers_a_note_on_all_of_it() {
    let mut app = opened();
    let file = app.workspace.opened.as_mut().expect("the open file");
    file.new.holding(Selection {
        anchor: Caret::new(0, 0),
        head: Caret::new(2, 1),
    });
    let mut ui = on_diff();
    ui.session.view = DiffView::Editor;
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
fn picking_the_note_opens_an_empty_row_to_type_in() {
    let app = opened();
    let mut ui = on_diff();
    ui.session.view = DiffView::Editor;
    asked(&app, &mut ui, 1);
    let commands = pick(&app, &mut ui);
    assert!(commands.is_empty(), "nothing is asked of the app yet");
    assert_eq!(
        ui.session.noting.as_ref().map(|one| one.anchor.clone()),
        Some(Anchor::line("src/lib.rs", 1))
    );
    assert!(ui.menu().is_none(), "the menu shuts behind it");
}

#[test]
fn the_words_typed_draw_in_the_row_under_the_line() {
    let app = opened();
    let mut ui = on_diff();
    ui.session.view = DiffView::Editor;
    ui.session.noting = Some(Noting::new(Anchor::line("src/lib.rs", 0)));
    typed("leaks", &mut ui, &app);
    let drawn = row_texts(&app, &ui);
    assert!(
        drawn.iter().any(|one| one.contains("leaks")),
        "the words stand in the surface: {drawn:?}"
    );
    let at = |text: &str| drawn.iter().position(|one| one.contains(text));
    assert!(
        at("leaks") < at("TWO"),
        "above the line under it: {drawn:?}"
    );
}

#[test]
fn a_note_being_typed_takes_a_row_of_its_own() {
    let app = opened();
    let mut ui = on_diff();
    ui.session.view = DiffView::Editor;
    let rows = crate::views::session::diff::rows_of(&app, &ui);
    ui.session.noting = Some(Noting::new(Anchor::line("src/lib.rs", 0)));
    assert_eq!(
        crate::views::session::diff::rows_of(&app, &ui),
        rows + 1,
        "the note being typed takes its room"
    );
}

#[test]
fn the_words_leave_a_note_when_the_keyboard_says_so() {
    let app = opened();
    let mut ui = on_diff();
    ui.session.view = DiffView::Editor;
    ui.session.noting = Some(Noting::new(Anchor::line("src/lib.rs", 1)));
    typed("issue: this leaks", &mut ui, &app);
    let commands = press(Key::Enter, &mut ui, &app);
    assert!(ui.session.noting.is_none(), "the row closes behind it");
    let delivery::Command::Note(act) = one_command(commands) else {
        panic!("a note is asked for");
    };
    assert_eq!(
        act,
        groove_controllers::delivery::NoteAct::Create {
            anchor: Anchor::line("src/lib.rs", 1),
            content: "issue: this leaks".into(),
            author: crate::views::session::diff::AUTHOR.into(),
        }
    );
}

#[test]
fn nothing_typed_leaves_no_note() {
    let app = opened();
    let mut ui = on_diff();
    ui.session.noting = Some(Noting::new(Anchor::line("src/lib.rs", 1)));
    let commands = press(Key::Enter, &mut ui, &app);
    assert!(commands.is_empty(), "{commands:?}");
    assert!(ui.session.noting.is_none());
}

#[test]
fn escape_drops_the_words_and_asks_nothing() {
    let app = opened();
    let mut ui = on_diff();
    ui.session.noting = Some(Noting::new(Anchor::line("src/lib.rs", 1)));
    typed("issue", &mut ui, &app);
    let commands = press(Key::Escape, &mut ui, &app);
    assert!(commands.is_empty(), "{commands:?}");
    assert!(ui.session.noting.is_none());
}

#[test]
fn what_is_pasted_goes_into_the_note() {
    let app = opened();
    let mut ui = on_diff();
    ui.session.noting = Some(Noting::new(Anchor::line("src/lib.rs", 1)));
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

/// A note of this session's own, on line 1.
fn own() -> groove_types::Note {
    groove_types::Note {
        origin: groove_types::NoteOrigin::Local(groove_types::AnnotationId::new("n1")),
        anchor: Some(Anchor::line("src/lib.rs", 1)),
        resolved: false,
        said: vec![groove_types::Said {
            author: "you".into(),
            body: "issue: this leaks".into(),
            at: groove_types::Timestamp::new(0),
        }],
    }
}

/// A click on one of the note's own buttons.
fn on_button(
    app: &AppState,
    ui: &mut Ui,
    button: crate::hit::NoteButton,
) -> Vec<groove_controllers::Command> {
    let (_, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let id = groove_types::NoteOrigin::Local(groove_types::AnnotationId::new("n1"));
    let rect = hits
        .rect_of(&Target::Note(id, button))
        .expect("the button is drawn");
    click(rect, ui, app, &hits)
}

#[test]
fn resolve_asks_for_the_note_to_be_resolved() {
    let mut app = opened();
    app.delivery.shown = vec![own()];
    let mut ui = on_diff();
    let commands = on_button(&app, &mut ui, crate::hit::NoteButton::Resolve);
    assert_eq!(
        one_command(commands),
        delivery::Command::Note(groove_controllers::delivery::NoteAct::Resolve {
            id: groove_types::AnnotationId::new("n1")
        })
    );
}

#[test]
fn a_resolved_note_offers_to_open_again() {
    let mut app = opened();
    let mut note = own();
    note.resolved = true;
    app.delivery.shown = vec![note];
    let mut ui = on_diff();
    let commands = on_button(&app, &mut ui, crate::hit::NoteButton::Resolve);
    assert_eq!(
        one_command(commands),
        delivery::Command::Note(groove_controllers::delivery::NoteAct::Reopen {
            id: groove_types::AnnotationId::new("n1")
        })
    );
}

#[test]
fn delete_asks_for_the_note_to_go() {
    let mut app = opened();
    app.delivery.shown = vec![own()];
    let mut ui = on_diff();
    let commands = on_button(&app, &mut ui, crate::hit::NoteButton::Delete);
    assert_eq!(
        one_command(commands),
        delivery::Command::Note(groove_controllers::delivery::NoteAct::Delete {
            id: groove_types::AnnotationId::new("n1")
        })
    );
}

#[test]
fn edit_opens_the_note_with_its_own_words_in_it() {
    let mut app = opened();
    app.delivery.shown = vec![own()];
    let mut ui = on_diff();
    let commands = on_button(&app, &mut ui, crate::hit::NoteButton::Edit);
    assert!(commands.is_empty(), "nothing is asked of the app yet");
    let noting = ui.session.noting.as_ref().expect("the row opens");
    assert_eq!(noting.said(), "issue: this leaks");
    assert_eq!(noting.anchor, Anchor::line("src/lib.rs", 1));
}

#[test]
fn what_is_typed_over_a_note_writes_that_note_again() {
    let mut app = opened();
    app.delivery.shown = vec![own()];
    let mut ui = on_diff();
    on_button(&app, &mut ui, crate::hit::NoteButton::Edit);
    if let Some(noting) = ui.session.noting.as_mut() {
        noting.field.set("nitpick: name it");
    }
    let commands = press(Key::Enter, &mut ui, &app);
    assert_eq!(
        one_command(commands),
        delivery::Command::Note(groove_controllers::delivery::NoteAct::Update {
            id: groove_types::AnnotationId::new("n1"),
            content: "nitpick: name it".into()
        })
    );
}

#[test]
fn a_line_that_already_carries_a_note_takes_no_other() {
    let mut app = opened();
    app.delivery.shown = vec![own()];
    let mut ui = on_diff();
    ui.session.view = DiffView::Editor;
    asked(&app, &mut ui, 1);
    assert!(
        ui.menu().is_none(),
        "one note a line: {:?}",
        ui.menu().map(|menu| &menu.of)
    );
}

#[test]
fn a_line_beside_a_noted_one_still_takes_a_note() {
    let mut app = opened();
    app.delivery.shown = vec![own()];
    let mut ui = on_diff();
    ui.session.view = DiffView::Editor;
    asked(&app, &mut ui, 0);
    assert_eq!(
        ui.menu().map(|menu| &menu.of),
        Some(&crate::Of::Line {
            path: "src/lib.rs".into(),
            lines: (0, 0),
        })
    );
}

#[test]
fn a_selection_that_runs_into_a_note_takes_no_note() {
    let mut app = opened();
    app.delivery.shown = vec![own()];
    let file = app.workspace.opened.as_mut().expect("the open file");
    file.new.holding(Selection {
        anchor: Caret::new(0, 0),
        head: Caret::new(1, 1),
    });
    let mut ui = on_diff();
    ui.session.view = DiffView::Editor;
    asked(&app, &mut ui, 0);
    assert!(ui.menu().is_none(), "the note on line 1 is inside it");
}

#[test]
fn a_note_written_again_is_typed_in_its_own_place() {
    let mut app = opened();
    app.delivery.shown = vec![own()];
    let mut ui = on_diff();
    ui.session.view = DiffView::Editor;
    let with_note = crate::views::session::diff::rows_of(&app, &ui);
    on_button(&app, &mut ui, crate::hit::NoteButton::Edit);
    assert_eq!(
        crate::views::session::diff::rows_of(&app, &ui),
        with_note - 1,
        "the typed row stands in place of the note and its buttons"
    );
}

#[test]
fn hovering_one_notes_button_leaves_the_others_alone() {
    let mut app = opened();
    let mut second = own();
    second.origin = groove_types::NoteOrigin::Local(groove_types::AnnotationId::new("n2"));
    second.anchor = Some(Anchor::line("src/lib.rs", 2));
    app.delivery.shown = vec![own(), second];
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let mut ui = on_diff();
    ui.hover = Some(Target::Note(
        groove_types::NoteOrigin::Local(groove_types::AnnotationId::new("n1")),
        crate::hit::NoteButton::Edit,
    ));
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let lit = frame
        .layers()
        .iter()
        .flat_map(|layer| layer.quads.iter())
        .filter(|quad| quad.color == styles.action())
        .count();
    assert_eq!(lit, 1, "one button of one note");
}

#[test]
fn a_noted_line_takes_no_other_note_in_the_diff_views() {
    for view_kind in [DiffView::Inline, DiffView::Split] {
        let mut app = opened();
        app.delivery.shown = vec![own()];
        let mut ui = on_diff();
        ui.session.view = view_kind;
        let row = noted_row(&app, &ui);
        asked(&app, &mut ui, row);
        assert!(
            ui.menu().is_none(),
            "{view_kind:?} offers no second note: {:?}",
            ui.menu().map(|menu| &menu.of)
        );
    }
}

/// The surface row the note's own line stands on.
fn noted_row(app: &AppState, ui: &Ui) -> usize {
    let total = crate::views::session::diff::rows_of(app, ui);
    (0..total)
        .find(|row| {
            crate::views::session::diff::line_at(app, ui, ui.session.view, *row)
                == Some(("src/lib.rs".to_string(), 1))
        })
        .expect("the note's line")
}
