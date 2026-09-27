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
        s.workspace.opened.is_some()
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
                .opened
                .as_ref()
                .is_some_and(|open| open.new.lines() == wanted)
        });
        total += started.elapsed();
    }
    println!(
        "a 300-line file saved: {:?} from write to state",
        total / RUNS
    );
}
