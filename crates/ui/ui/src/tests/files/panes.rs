//! The sidebar's three lists: the strip that picks one, and what the notes list shows.

use groove_types::{Anchor, AnnotationId, Note, NoteOrigin, Said, Timestamp};

use super::*;
use crate::tests::sidebar_ui;
use crate::views::session::Pane;

fn note(path: &str, line: u32, body: &str) -> Note {
    Note {
        origin: NoteOrigin::Local(AnnotationId::new(path)),
        anchor: Some(Anchor::line(path, line)),
        resolved: false,
        said: vec![Said {
            author: "you".into(),
            body: body.into(),
            at: Timestamp::new(0),
        }],
    }
}

/// The fixture with those notes and the notes list up.
fn noting(notes: Vec<Note>) -> (AppState, Ui) {
    let mut app = with_files(&["src/lib.rs"]);
    app.delivery.shown = notes;
    let mut ui = sidebar_ui();
    ui.session.pane = Pane::Notes;
    (app, ui)
}

/// Every text the sidebar drew.
fn in_sidebar(app: &AppState, ui: &Ui) -> Vec<String> {
    let (frame, _) = view(app, ui, window(), &mut Fonts::embedded());
    let rect = crate::layout::Layout::of(window(), ui).sidebar;
    frame.layers()[0]
        .texts
        .iter()
        .filter(|run| run.x >= rect.x && run.x < rect.right())
        .map(|run| run.text.clone())
        .collect()
}

#[test]
fn the_strip_names_the_three_lists_and_counts_them() {
    let (app, ui) = noting(vec![note("src/lib.rs", 11, "issue: this leaks")]);
    let drawn = in_sidebar(&app, &ui);
    assert!(
        drawn.iter().any(|one| one.starts_with("changed")),
        "{drawn:?}"
    );
    assert!(drawn.iter().any(|one| one == "commits"), "{drawn:?}");
    assert!(drawn.iter().any(|one| one == "notes · 1"), "{drawn:?}");
}

#[test]
fn a_resolved_note_is_not_counted() {
    let mut one = note("src/lib.rs", 11, "issue: this leaks");
    one.resolved = true;
    let (app, ui) = noting(vec![one]);
    let drawn = in_sidebar(&app, &ui);
    assert!(drawn.iter().any(|one| one == "notes"), "{drawn:?}");
}

#[test]
fn the_notes_list_says_the_file_the_line_and_the_words() {
    let (app, ui) = noting(vec![note("src/lib.rs", 11, "issue: this leaks")]);
    let drawn = in_sidebar(&app, &ui);
    assert!(drawn.iter().any(|one| one == "lib.rs 12"), "{drawn:?}");
    assert!(
        drawn.iter().any(|one| one.starts_with("issue: this l")),
        "{drawn:?}"
    );
}

#[test]
fn a_thread_says_how_many_replies_it_holds() {
    let mut one = note("src/lib.rs", 11, "issue: this leaks");
    one.origin = NoteOrigin::Thread("t1".into());
    one.said.push(Said {
        author: "haoov".into(),
        body: "fixed".into(),
        at: Timestamp::new(1),
    });
    let (app, ui) = noting(vec![one]);
    let drawn = in_sidebar(&app, &ui);
    assert!(
        drawn.iter().any(|one| one.contains("+1")),
        "one reply: {drawn:?}"
    );
}

#[test]
fn the_notes_list_says_when_there_are_none() {
    let (app, ui) = noting(Vec::new());
    let drawn = in_sidebar(&app, &ui);
    assert!(
        drawn.iter().any(|one| one == "no notes on this session"),
        "{drawn:?}"
    );
}

#[test]
fn picking_the_notes_list_reads_the_notes() {
    let app = with_files(&["src/lib.rs"]);
    let mut ui = sidebar_ui();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let strip = hits
        .rect_of(&Target::Pane(Pane::Notes))
        .expect("the strip names the notes");
    click(strip, &mut ui, &app, &hits);
    assert_eq!(ui.session.pane, Pane::Notes);
    let asked = crate::render::layout_commands(&app, &ui, window());
    assert!(
        asked.iter().any(|one| matches!(
            one,
            groove_controllers::Command::Delivery(groove_controllers::delivery::Command::GetNotes)
        )),
        "the frame asks for them: {asked:?}"
    );
}

#[test]
fn the_notes_are_read_once_a_session() {
    let mut app = with_files(&["src/lib.rs"]);
    let mut ui = sidebar_ui();
    ui.session.pane = Pane::Notes;
    let asks = |app: &AppState, ui: &Ui| {
        crate::render::layout_commands(app, ui, window())
            .iter()
            .filter(|one| {
                matches!(
                    one,
                    groove_controllers::Command::Delivery(
                        groove_controllers::delivery::Command::GetNotes
                    )
                )
            })
            .count()
    };
    assert_eq!(asks(&app, &ui), 1, "nothing is held yet");
    let session = app.session.selected.clone().expect("a session");
    app.delivery.reading(&session);
    assert_eq!(asks(&app, &ui), 0, "the read is out or landed");
    app.session.selected = Some(groove_types::SessionId::new("b"));
    assert_eq!(asks(&app, &ui), 1, "another session brings its own");
}

#[test]
fn the_commit_box_stands_under_the_files_list_alone() {
    let ui = sidebar_ui();
    assert!(
        !crate::layout::Layout::of(window(), &ui).commit.is_empty(),
        "the files list has it"
    );
    let mut notes = sidebar_ui();
    notes.session.pane = Pane::Notes;
    assert!(
        crate::layout::Layout::of(window(), &notes)
            .commit
            .is_empty(),
        "the notes list gives its room to the list"
    );
}

#[test]
fn a_note_of_the_list_opens_the_line_it_stands_on() {
    let (app, mut ui) = noting(vec![note("src/lib.rs", 11, "issue: this leaks")]);
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let line = hits.rect_of(&Target::NoteAt(0)).expect("the note's row");
    let commands = click(line, &mut ui, &app, &hits);
    assert_eq!(ui.session.tab, crate::views::session::Tab::Files);
    let opened = commands.iter().find_map(|one| match one {
        groove_controllers::Command::Workspace(
            groove_controllers::workspace::Command::OpenFile { path, at },
        ) => Some((path.clone(), *at)),
        _ => None,
    });
    let (path, at) = opened.expect("the file is opened");
    assert_eq!(path, "src/lib.rs");
    assert_eq!(
        at.map(|held| held.head.line),
        Some(11),
        "at the note's own line"
    );
}

#[test]
fn the_strip_is_the_same_widget_the_workspace_tabs_are() {
    let (app, ui) = noting(vec![note("src/lib.rs", 11, "issue: this leaks")]);
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let tab = hits
        .rect_of(&Target::Pane(Pane::Notes))
        .expect("the list up");
    let lit = frame
        .layers()
        .iter()
        .flat_map(|layer| layer.quads.iter())
        .any(|quad| quad.color == styles.raised() && quad.rect == tab);
    assert!(lit, "the one up stands on its own ground");
}

#[test]
fn the_search_bar_stands_only_while_it_is_used() {
    let app = with_files(&["src/lib.rs"]);
    let bare = sidebar_ui();
    let (_, hits) = view(&app, &bare, window(), &mut Fonts::embedded());
    let strip = hits.rect_of(&Target::Pane(Pane::Files)).expect("the strip");
    let sidebar = crate::layout::Layout::of(window(), &bare).sidebar;
    assert_eq!(strip.y, sidebar.y, "the strip starts at the top");

    let mut typing = sidebar_ui();
    typing.session.bar.open(crate::views::session::Term::Path);
    let (_, hits) = view(&app, &typing, window(), &mut Fonts::embedded());
    let under = hits.rect_of(&Target::Pane(Pane::Files)).expect("the strip");
    assert!(under.y > strip.y, "the bar pushes it down");
}

/// A thread the forge holds, on a line or on the MR itself.
fn thread(id: &str, line: Option<u32>, resolved: bool) -> Note {
    Note {
        origin: NoteOrigin::Thread(id.into()),
        anchor: line.map(|at| Anchor::line("src/lib.rs", at)),
        resolved,
        said: vec![Said {
            author: "reviewer".into(),
            body: format!("said on {id}"),
            at: Timestamp::new(0),
        }],
    }
}

#[test]
fn the_notes_list_holds_the_session_s_own_and_the_open_threads_alone() {
    let (app, ui) = noting(vec![
        thread("pipeline", None, false),
        thread("closed", Some(3), true),
        note("src/lib.rs", 11, "issue: this leaks"),
        thread("open", Some(20), false),
    ]);
    let drawn = in_sidebar(&app, &ui);
    let said = |text: &str| drawn.iter().any(|one| one.contains(text));
    assert!(said("issue: this leaks"), "the session's own: {drawn:?}");
    assert!(said("said on open"), "a thread still open: {drawn:?}");
    assert!(!said("said on pipeline"), "not a comment on the MR");
    assert!(!said("said on closed"), "not a resolved thread");
    assert!(drawn.iter().any(|one| one == "notes · 2"), "{drawn:?}");
}

#[test]
fn a_row_left_after_the_others_are_left_out_opens_its_own_note() {
    let (app, mut ui) = noting(vec![
        thread("pipeline", None, false),
        note("src/lib.rs", 11, "issue: this leaks"),
    ]);
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(
        hits.rect_of(&Target::NoteAt(0)).is_none(),
        "the comment has no row"
    );
    let line = hits.rect_of(&Target::NoteAt(1)).expect("the note's row");
    let commands = click(line, &mut ui, &app, &hits);
    let at = commands.iter().find_map(|one| match one {
        groove_controllers::Command::Workspace(
            groove_controllers::workspace::Command::OpenFile { at, .. },
        ) => *at,
        _ => None,
    });
    assert_eq!(at.map(|held| held.head.line), Some(11), "the note it names");
}

#[test]
fn a_session_s_notes_are_read_whichever_list_is_up() {
    let app = with_files(&["src/lib.rs"]);
    let ui = sidebar_ui();
    assert_ne!(ui.session.pane, Pane::Notes);
    let asked = crate::render::layout_commands(&app, &ui, window());
    assert!(
        asked.iter().any(|one| matches!(
            one,
            groove_controllers::Command::Delivery(groove_controllers::delivery::Command::GetNotes)
        )),
        "the surface draws them too: {asked:?}"
    );
}
