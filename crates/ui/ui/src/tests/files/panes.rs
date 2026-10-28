//! The sidebar's three lists: the strip that picks one, and what the notes list shows.

use groove_types::{Anchor, AnnotationId, Note, NoteOrigin, Said, Timestamp};

mod listed;

use super::*;
use crate::tests::sidebar_ui;
use crate::views::session::{Pane, Term};

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
    let said = |pane| tab_said(&app, &ui, pane);
    assert_eq!(said(Pane::Files).as_deref(), Some("1"), "the changed file");
    assert_eq!(said(Pane::Commits).as_deref(), Some(""), "a mark alone");
    assert_eq!(said(Pane::Notes).as_deref(), Some("1"), "the open note");
}

#[test]
fn a_resolved_note_is_not_counted() {
    let mut one = note("src/lib.rs", 11, "issue: this leaks");
    one.resolved = true;
    let (app, ui) = noting(vec![one]);
    assert_eq!(tab_said(&app, &ui, Pane::Notes).as_deref(), Some(""));
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
    let asked = crate::render::frame_commands(&app, &ui, window());
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
        crate::render::frame_commands(app, ui, window())
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
    let held = frame.layers()[0]
        .quads
        .iter()
        .any(|quad| quad.color == styles.glass_active() && quad.rect == tab);
    assert!(held, "the one up stands on a rounded held ground");
    let other = hits
        .rect_of(&Target::Pane(Pane::Files))
        .expect("another list");
    let quiet = frame.layers()[0]
        .texts
        .iter()
        .find(|run| other.contains(run.x, run.y + run.height / 2.0))
        .map(|run| run.style.color);
    assert_eq!(
        quiet,
        Some(styles.color(groove_ui_kit::base::style::Role::Muted)),
        "the others are muted"
    );
}

#[test]
fn one_search_field_always_stands_over_the_strip_and_switches_its_term() {
    let app = with_files(&["src/lib.rs"]);
    let mut ui = sidebar_ui();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let field = hits
        .rect_of(&Target::Term(Term::Path))
        .expect("the path field, unasked");
    let strip = hits.rect_of(&Target::Pane(Pane::Files)).expect("the strip");
    assert!(field.bottom() <= strip.y, "the field above the strip");
    let switch = hits
        .rect_of(&Target::Term(Term::Text))
        .expect("what switches to text");
    click(switch, &mut ui, &app, &hits);
    assert_eq!(ui.session.bar.typing, Some(Term::Text));
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(
        hits.rect_of(&Target::Term(Term::Path)).is_some(),
        "and back to the path"
    );
    let after = hits.rect_of(&Target::Pane(Pane::Files)).expect("the strip");
    assert_eq!(after.y, strip.y, "one field, whichever term it holds");
}

/// A thread the forge holds, on a line or on the MR itself.
#[test]
fn a_session_s_notes_are_read_whichever_list_is_up() {
    let app = with_files(&["src/lib.rs"]);
    let ui = sidebar_ui();
    assert_ne!(ui.session.pane, Pane::Notes);
    let asked = crate::render::frame_commands(&app, &ui, window());
    assert!(
        asked.iter().any(|one| matches!(
            one,
            groove_controllers::Command::Delivery(groove_controllers::delivery::Command::GetNotes)
        )),
        "the surface draws them too: {asked:?}"
    );
}

#[test]
fn a_narrow_tab_keeps_its_mark_whole_inside_it() {
    let app = with_files(&["src/lib.rs"]);
    let mut ui = sidebar_ui();
    ui.split.sidebar = groove_ui_kit::base::tokens::SIDEBAR_MIN;
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    for pane in Pane::ALL {
        let tab = hits.rect_of(&Target::Pane(pane)).expect("the tab");
        let mark = frame.layers()[0]
            .icons
            .iter()
            .find(|icon| icon.icon == pane.mark().shape() && tab.contains(icon.rect.x, icon.rect.y))
            .expect("its mark");
        assert!(mark.rect.right() <= tab.right(), "{pane:?} whole");
    }
}
