//! The notes list: its filter, its files, each note on two lines, and where a click lands.

use super::*;
use crate::views::session::{Showing, Tab};

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
fn a_note_reads_on_two_lines_under_its_file() {
    let (app, ui) = noting(vec![note("src/lib.rs", 11, "issue (blocking): this leaks")]);
    let drawn = in_sidebar(&app, &ui);
    for text in [
        "lib.rs",
        "src",
        "issue",
        "this leaks",
        "ME",
        "me",
        "local",
        "L12",
    ] {
        assert!(drawn.iter().any(|one| one == text), "{text}: {drawn:?}");
    }
    assert!(
        !drawn.iter().any(|one| one.contains("issue (blocking)")),
        "{drawn:?}"
    );
}

#[test]
fn a_thread_says_how_many_replies_it_holds() {
    let mut one = thread("t1", Some(11), false);
    one.said.push(Said {
        author: "haoov".into(),
        body: "fixed".into(),
        at: Timestamp::new(1),
    });
    let (app, ui) = noting(vec![one]);
    let drawn = in_sidebar(&app, &ui);
    assert!(drawn.iter().any(|one| one == "1 reply"), "{drawn:?}");
    assert!(drawn.iter().any(|one| one == "MR"), "{drawn:?}");
}

#[test]
fn the_filter_counts_open_resolved_and_all_and_shows_the_one_picked() {
    let (app, mut ui) = noting(vec![
        thread("pipeline", None, false),
        thread("closed", Some(3), true),
        note("src/lib.rs", 11, "issue: this leaks"),
    ]);
    let drawn = in_sidebar(&app, &ui);
    for text in ["open · 2", "resolved · 1", "all · 3", "notes · 2"] {
        assert!(drawn.iter().any(|one| one == text), "{text}: {drawn:?}");
    }
    let said = |drawn: &[String], text: &str| drawn.iter().any(|one| one == text);
    assert!(said(&drawn, "this leaks") && said(&drawn, "said on pipeline"));
    assert!(!said(&drawn, "said on closed"), "open alone: {drawn:?}");
    let at = |drawn: &[String], text: &str| drawn.iter().position(|one| one == text);
    assert!(
        at(&drawn, "on the MR") > at(&drawn, "this leaks"),
        "the MR's own last"
    );
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let resolved = hits
        .rect_of(&Target::Showing(Showing::Resolved))
        .expect("the filter");
    click(resolved, &mut ui, &app, &hits);
    let drawn = in_sidebar(&app, &ui);
    assert!(
        said(&drawn, "said on closed") && !said(&drawn, "this leaks"),
        "{drawn:?}"
    );
}

#[test]
fn an_empty_list_says_what_it_would_hold() {
    let (app, mut ui) = noting(Vec::new());
    assert!(
        in_sidebar(&app, &ui)
            .iter()
            .any(|one| one == "No open notes")
    );
    ui.session.showing = Showing::Resolved;
    assert!(
        in_sidebar(&app, &ui)
            .iter()
            .any(|one| one == "Nothing resolved yet")
    );
}

#[test]
fn the_files_tab_offers_the_notes_too_and_a_click_opens_the_file_at_the_note() {
    let (app, mut ui) = noting(vec![note("src/lib.rs", 11, "issue: this leaks")]);
    ui.session.tab = Tab::Files;
    let drawn = in_sidebar(&app, &ui);
    assert!(drawn.iter().any(|one| one == "files"), "{drawn:?}");
    assert!(
        !drawn.iter().any(|one| one.starts_with("commits")),
        "{drawn:?}"
    );
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let line = hits.rect_of(&Target::NoteAt(0)).expect("the note's row");
    let commands = click(line, &mut ui, &app, &hits);
    let at = commands.iter().find_map(|one| match one {
        groove_controllers::Command::Workspace(
            groove_controllers::workspace::Command::OpenFile { at, .. },
        ) => *at,
        _ => None,
    });
    assert_eq!(
        at.map(|held| held.head.line),
        Some(11),
        "in the file, at its line"
    );
    assert_eq!(ui.session.tab, Tab::Files);
}

#[test]
fn a_click_on_a_resolved_note_opens_it_from_behind_its_tick() {
    let (app, mut ui) = noting(vec![thread("closed", Some(3), true)]);
    ui.session.showing = Showing::Resolved;
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let line = hits.rect_of(&Target::NoteAt(0)).expect("the note's row");
    click(line, &mut ui, &app, &hits);
    assert_eq!(ui.session.unfolded, [NoteOrigin::Thread("closed".into())]);
}
