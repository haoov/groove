use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_types::{Session, SessionId, SessionKind, SessionState, Timestamp};

use crate::input::{Input, Key, Modifiers, handle};
use crate::palette::{Palette, entries, matching};
use crate::{Metrics, Ui, view};

fn open(id: &str, title: &str) -> Open {
    Open {
        session: Session {
            id: SessionId::new(id),
            title: title.into(),
            kind: SessionKind::Explorer,
            created_at: Timestamp::new(0),
        },
        state: SessionState::default(),
        worktrees: vec![],
        delivery: vec![],
    }
}

fn app() -> AppState {
    let mut app = AppState::default();
    app.session.open.push(open("a", "Alpha"));
    app.session.open.push(open("b", "Beta"));
    app.session.selected = Some(SessionId::new("a"));
    app
}

#[test]
fn every_entry_is_a_controller_function() {
    let rows = entries(&app());
    let ids: Vec<&str> = rows.iter().map(|e| e.id()).collect();
    assert_eq!(
        ids,
        ["session.open_explorer", "session.close", "session.select"]
    );
    assert!(rows.iter().all(|e| e.id().contains('.')));
    assert_eq!(
        entries(&AppState::default()).len(),
        1,
        "nothing selected: nothing to close or switch to"
    );
}

#[test]
fn the_query_filters_by_word_and_ranks_by_position() {
    let rows = matching(entries(&app()), "beta");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].label, "Switch to Beta");
    let rows = matching(entries(&app()), "SESSION cl");
    assert_eq!(rows[0].label, "Close session");
    assert!(matching(entries(&app()), "zzz").is_empty());
    assert_eq!(matching(entries(&app()), "").len(), 3);
}

#[test]
fn typing_moving_and_enter_run_the_selected_row() {
    let app = app();
    let mut palette = Palette::default();
    for c in "switch".chars() {
        assert!(palette.key(Key::Char(c), &app).is_none());
    }
    assert_eq!(palette.rows(&app).len(), 1);
    palette.key(Key::Backspace, &app);
    palette.key(Key::Backspace, &app);
    palette.key(Key::Backspace, &app);
    palette.key(Key::Backspace, &app);
    palette.key(Key::Backspace, &app);
    palette.key(Key::Backspace, &app);
    assert_eq!(palette.query, "");
    palette.key(Key::Down, &app);
    palette.key(Key::Down, &app);
    palette.key(Key::Down, &app);
    assert_eq!(palette.selected, 2, "clamped to the last row");
    palette.key(Key::Up, &app);
    let command = palette.key(Key::Enter, &app).unwrap();
    assert_eq!(command.id(), "session.close");
}

#[test]
fn the_palette_closes_on_enter_and_draws_its_rows() {
    let app = app();
    let mut ui = Ui::default();
    let chord = Modifiers {
        ctrl: true,
        shift: true,
        alt: false,
    };
    handle(
        Input::Key {
            key: Key::Char('p'),
            mods: chord,
        },
        &mut ui,
        &app,
    );
    let frame = view(
        &app,
        &ui,
        Metrics {
            size: groove_gfx::Size::new(1280, 800),
            scale: 1.0,
            cell: groove_gfx::CellSize {
                width: 8.0,
                height: 17.0,
            },
        },
    );
    let texts: Vec<String> = frame.layers()[1]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
    assert!(texts.iter().any(|t| t == "New explorer"));
    assert!(texts.iter().any(|t| t == "Switch to Beta"));
    let run = handle(
        Input::Key {
            key: Key::Enter,
            mods: Modifiers::default(),
        },
        &mut ui,
        &app,
    )
    .unwrap();
    assert_eq!(run.id(), "session.open_explorer");
    assert!(ui.palette.is_none());
}
