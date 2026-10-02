//! The blame after the line the caret rests on: when it is asked for, what it says, a click.

use super::*;
use crate::input::rest;
use groove_controllers::{Command, workspace};
use groove_types::{BlameLine, Caret, Edit, Motion, Timestamp};
use groove_ui_kit::base::ctx::Metrics;
use groove_ui_kit::base::tokens::REST_MS;

const DAY: i64 = 86_400;

/// The open file in the file view, the caret on its second line.
fn resting() -> (AppState, Ui) {
    let mut app = opened();
    let open = app.workspace.active_mut().expect("the open file");
    open.new.edit(&Edit::Move(Motion::To(Caret::new(1, 0))));
    let mut ui = on_diff();
    ui.session.tab = Tab::Files;
    ui.focus = crate::Focus::Workspace;
    (app, ui)
}

fn line(at: u32, uncommitted: bool) -> BlameLine {
    BlameLine {
        line: at,
        sha: format!("{at}abcdef0123456789"),
        short_sha: format!("{at}abcdef"),
        author: "Ada".into(),
        at: Timestamp::new(0),
        summary: "the change".into(),
        uncommitted,
    }
}

/// The blame of the open file, kept for the read it has now.
fn blamed(app: &mut AppState, lines: Vec<BlameLine>) {
    let worktree = app
        .session
        .selected_worktree()
        .expect("a worktree")
        .id
        .clone();
    let read = app.workspace.read_of("src/lib.rs");
    let blames = &mut app.workspace.blames;
    blames.asked(&worktree, "src/lib.rs", read);
    blames.took(&worktree, "src/lib.rs".into(), read, lines);
}

/// The window `after` milliseconds into the run, five days past the epoch.
fn at(after: u64) -> Metrics {
    Metrics {
        tick: after,
        now: Timestamp::new(5 * DAY),
        ..window()
    }
}

fn texts(app: &AppState, ui: &Ui, metrics: Metrics) -> Vec<String> {
    let (frame, _) = view(app, ui, metrics, &mut Fonts::embedded());
    frame.layers()[0]
        .texts
        .iter()
        .map(|run| run.text.clone())
        .collect()
}

#[test]
fn the_blame_is_asked_for_once_the_caret_has_rested_long_enough() {
    let (app, mut ui) = resting();
    assert_eq!(rest(&mut ui, &app, 100), None, "the count starts");
    assert_eq!(rest(&mut ui, &app, 100 + REST_MS - 1), None);
    let asked = rest(&mut ui, &app, 100 + REST_MS);
    let wanted = workspace::Command::Blame {
        path: "src/lib.rs".into(),
    };
    assert_eq!(asked, Some(Command::Workspace(wanted)));
}

#[test]
fn an_edit_starts_the_count_again() {
    let (mut app, mut ui) = resting();
    rest(&mut ui, &app, 0);
    let open = app.workspace.active_mut().expect("the open file");
    open.new.edit(&Edit::Insert("x".into()));
    assert_eq!(rest(&mut ui, &app, REST_MS), None, "the buffer changed");
    assert!(rest(&mut ui, &app, 2 * REST_MS).is_some());
}

#[test]
fn a_kept_blame_is_not_asked_for_again() {
    let (mut app, mut ui) = resting();
    blamed(&mut app, vec![line(0, false), line(1, false)]);
    rest(&mut ui, &app, 0);
    assert_eq!(rest(&mut ui, &app, REST_MS), None);
}

#[test]
fn the_rested_line_says_who_changed_it_when_and_in_which_commit() {
    let (mut app, mut ui) = resting();
    blamed(&mut app, vec![line(0, false), line(1, false)]);
    rest(&mut ui, &app, 0);
    let said = "Ada, 5d ago · 1abcdef".to_string();
    assert!(
        !texts(&app, &ui, at(REST_MS - 1)).contains(&said),
        "not yet"
    );
    assert!(texts(&app, &ui, at(REST_MS)).contains(&said));
}

#[test]
fn a_line_not_committed_says_so() {
    let (mut app, mut ui) = resting();
    blamed(&mut app, vec![line(0, false), line(1, true)]);
    rest(&mut ui, &app, 0);
    let said = "You · not committed yet".to_string();
    assert!(texts(&app, &ui, at(REST_MS)).contains(&said));
}

#[test]
fn a_click_on_the_blame_shows_its_commit() {
    let (mut app, mut ui) = resting();
    blamed(&mut app, vec![line(0, false), line(1, false)]);
    rest(&mut ui, &app, 0);
    let (_, hits) = view(&app, &ui, at(REST_MS), &mut Fonts::embedded());
    let sha = "1abcdef0123456789".to_string();
    let rect = hits
        .rect_of(&Target::Blamed(sha.clone()))
        .expect("the blame takes a click");
    let sha_wide = "1abcdef".len() as f32 * hits.chars().advance;
    assert!((rect.w - sha_wide).abs() < 0.5, "only the sha: {rect:?}");
    let commands = click(rect, &mut ui, &app, &hits);
    assert_eq!(
        commands,
        [Command::Workspace(workspace::Command::OpenCommit { sha })]
    );
    assert_eq!(ui.session.tab, Tab::Diff, "the commit shows as a change");
}
