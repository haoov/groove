//! How long a write on disk takes to reach the state a frame draws. Run with
//! `cargo test --release -p groove-controllers perf -- --ignored --nocapture`.
#![allow(clippy::print_stdout)]

use std::path::Path;
use std::time::{Duration, Instant};

use crate::tests::fixture::{pooled_clone, until, worktree};
use crate::{AppState, Command as Cmd, Services, SyncSpawner, dispatch, workspace};

const RUNS: u32 = 5;

/// Drains until `done`, the way the window does when a continuation arrives.
fn settle(
    spawner: &SyncSpawner,
    services: &Services,
    state: &mut AppState,
    mut done: impl FnMut(&AppState) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(state) {
        spawner.drain(state, services);
        assert!(Instant::now() < deadline, "timed out");
        std::thread::sleep(Duration::from_micros(200));
    }
}

#[test]
#[ignore]
fn time_a_save_reaching_the_state() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    let file = Path::new(&dir).join("a.txt");

    let source: String = (0..300).map(|at| format!("line {at}\n")).collect();
    std::fs::write(&file, &source).unwrap();
    until(&spawner, &services, &mut state, |s| {
        !s.workspace.files.is_empty()
    });
    dispatch(
        Cmd::Workspace(workspace::Command::OpenFile {
            at: None,
            path: "a.txt".into(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.active().is_some()
    });

    let mut total = Duration::ZERO;
    for at in 0..RUNS {
        let extra: String = (0..=at).map(|n| format!("line {n} again\n")).collect();
        let text = format!("{source}{extra}");
        let wanted = 301 + at as usize;
        let started = Instant::now();
        std::fs::write(&file, &text).unwrap();
        settle(&spawner, &services, &mut state, |s| {
            s.workspace
                .active()
                .is_some_and(|open| open.new.lines() == wanted)
        });
        total += started.elapsed();
    }
    println!(
        "a 300-line file saved: {:?} from write to state",
        total / RUNS
    );
}

#[test]
#[ignore]
fn time_a_reload_over_unsaved_files() {
    for open in [1, 10, 30] {
        let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
        pooled_clone(home.path());
        let dir = worktree(&mut state, &services, &spawner);
        let source: String = (0..300).map(|at| format!("line {at}\n")).collect();
        for at in 0..open {
            std::fs::write(Path::new(&dir).join(format!("f{at}.txt")), &source).unwrap();
        }
        until(&spawner, &services, &mut state, |s| {
            s.workspace.files.len() == open
        });
        for at in 0..open {
            let path = format!("f{at}.txt");
            let command = workspace::Command::OpenFile { at: None, path };
            dispatch(Cmd::Workspace(command), &mut state, &services, &spawner);
            until(&spawner, &services, &mut state, |s| {
                s.workspace.active().is_some()
            });
            let edit = workspace::Command::Edit(groove_types::Edit::Insert("x".into()));
            dispatch(Cmd::Workspace(edit), &mut state, &services, &spawner);
        }
        settle(&spawner, &services, &mut state, |s| {
            s.workspace.deriving.is_empty()
        });
        assert_eq!(state.workspace.dirty().len(), open);

        let started = Instant::now();
        for _ in 0..RUNS {
            let load = Cmd::Workspace(workspace::Command::Load);
            dispatch(load, &mut state, &services, &spawner);
            settle(&spawner, &services, &mut state, |s| {
                s.pending.is_empty() && s.workspace.deriving.is_empty()
            });
        }
        println!(
            "{open:>3} unsaved files: {:?} a reload, rows derived again",
            started.elapsed() / RUNS
        );
    }
}
