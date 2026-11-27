//! The sidebar's three lists: the strip that picks one, and what the notes list shows.

use groove_types::{Anchor, Note, Said, Timestamp};

mod listed;

use super::*;
use crate::tests::sidebar_ui;
use crate::views::session::Pane;

fn note(path: &str, line: u32, body: &str) -> Note {
    Note {
        thread: path.into(),
        anchor: Some(Anchor::line(path, line)),
        resolved: false,
        said: vec![Said {
            author: "reviewer".into(),
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
    frame
        .layers()
        .iter()
        .flat_map(|layer| layer.texts.iter())
        .filter(|run| run.x >= rect.x && run.x < rect.right())
        .map(|run| run.text.clone())
        .collect()
}

#[test]
fn the_strip_names_the_three_lists_and_counts_them() {
    let (app, ui) = noting(vec![note("src/lib.rs", 11, "issue: this leaks")]);
    let said = |pane| tab_said(&app, &ui, pane);
    assert_eq!(
        said(Pane::Files).as_deref(),
        Some("FILES · 1"),
        "the changed file"
    );
    assert_eq!(
        said(Pane::Commits).as_deref(),
        Some("COMMITS"),
        "a name alone"
    );
    assert_eq!(
        said(Pane::Notes).as_deref(),
        Some("THREADS · 1"),
        "the open note"
    );
}

#[test]
fn a_resolved_note_is_not_counted() {
    let mut one = note("src/lib.rs", 11, "issue: this leaks");
    one.resolved = true;
    let (app, ui) = noting(vec![one]);
    assert_eq!(tab_said(&app, &ui, Pane::Notes).as_deref(), Some("THREADS"));
}

#[test]
fn picking_the_notes_list_puts_it_up() {
    let app = with_files(&["src/lib.rs"]);
    let mut ui = sidebar_ui();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let strip = hits
        .rect_of(&Target::Pane(Pane::Notes))
        .expect("the strip names the notes");
    click(strip, &mut ui, &app, &hits);
    assert_eq!(ui.session.pane, Pane::Notes);
}

#[test]
fn a_note_of_the_list_opens_the_line_it_stands_on() {
    let (app, mut ui) = noting(vec![note("src/lib.rs", 11, "issue: this leaks")]);
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let line = hits.rect_of(&Target::NoteAt(0)).expect("the note's row");
    let commands = click(line, &mut ui, &app, &hits);
    assert_eq!(ui.session.mode, crate::views::session::Mode::Diff);
    let focus = groove_controllers::workspace::Command::Focus {
        path: Some("src/lib.rs".into()),
    };
    assert_eq!(commands, [groove_controllers::Command::Workspace(focus)]);
    assert_eq!(
        ui.session.revealing,
        Some(("src/lib.rs".into(), 11)),
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
    let held = frame
        .layers()
        .iter()
        .flat_map(|layer| layer.quads.iter())
        .any(|quad| quad.color == styles.glass_chosen() && quad.rect == tab);
    assert!(held, "the one up stands on a rounded held ground");
    let other = hits
        .rect_of(&Target::Pane(Pane::Files))
        .expect("another list");
    let quiet = frame
        .layers()
        .iter()
        .flat_map(|layer| layer.texts.iter())
        .find(|run| other.contains(run.x, run.y + run.height / 2.0))
        .map(|run| run.style.color);
    assert_eq!(
        quiet,
        Some(styles.color(groove_ui_kit::base::style::Role::Faint)),
        "the others are faint"
    );
}
