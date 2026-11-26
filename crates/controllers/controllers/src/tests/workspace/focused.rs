//! One file of the change focused: it alone takes rows, through a read of the worktree again.

use super::*;
use crate::workspace_service::HEAD_ROWS;

#[test]
fn a_focused_file_alone_takes_rows_and_stays_focused_when_read_again() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    let write = |name: &str, text: &str| {
        std::fs::write(std::path::Path::new(&dir).join(name), text).unwrap();
    };
    write("a.txt", "one\ntwo\n");
    write("new.txt", "fresh\n");
    act(&mut state, &services, &spawner, workspace::Command::Load);
    until(&spawner, &services, &mut state, |s| {
        s.workspace.changes.files().len() == 2
    });
    let path = Some("new.txt".to_string());
    act(
        &mut state,
        &services,
        &spawner,
        workspace::Command::Focus { path },
    );
    assert_eq!(state.workspace.changes.rows(), 1 + HEAD_ROWS);
    assert_eq!(state.workspace.changes.head_of("a.txt"), None);

    write("new.txt", "fresh\nagain\n");
    act(&mut state, &services, &spawner, workspace::Command::Load);
    until(&spawner, &services, &mut state, |s| {
        s.workspace.changes.rows() == 2 + HEAD_ROWS
    });
    assert_eq!(state.workspace.changes.focused(), Some("new.txt"));

    act(
        &mut state,
        &services,
        &spawner,
        workspace::Command::Focus { path: None },
    );
    assert!(state.workspace.changes.head_of("a.txt").is_some());
}
