//! Taking a session away: what goes with it, and what a refusal leaves standing.

use crate::session::Command;
use crate::tests::fixture::{self, until};
use crate::tests::session::open_explorer;
use crate::{Command as Cmd, dispatch};

fn line(session: &groove_types::SessionId) -> groove_types::TimelineEvent {
    groove_types::TimelineEvent {
        session: session.clone(),
        at: groove_types::Timestamp::new(0),
        kind: groove_types::TimelineKind::Commit,
        subject: String::new(),
        payload: serde_json::Value::Null,
    }
}

#[test]
fn a_deleted_session_takes_its_feed_lines_and_launch_files() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    open_explorer(&mut state, &services, &spawner, "keep");
    open_explorer(&mut state, &services, &spawner, "drop");
    let ids: Vec<_> = state
        .session
        .open
        .iter()
        .map(|o| o.session.id.clone())
        .collect();
    until(&spawner, &services, &mut state, |s| {
        ids.iter().all(|id| s.agent.agent(id).is_some())
    });
    state.session.feed = ids.iter().map(line).collect();
    let launched = |id: &groove_types::SessionId| {
        home.path()
            .join("data/agent-launch")
            .join(format!("{id}.prompt.md"))
            .is_file()
    };
    assert!(launched(&ids[1]), "the launch wrote its files");

    let delete = Command::Delete {
        session: ids[1].clone(),
    };
    dispatch(Cmd::Session(delete), &mut state, &services, &spawner);
    spawner.drain(&mut state, &services);
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert_eq!(state.session.feed, [line(&ids[0])]);
    assert!(!launched(&ids[1]), "its launch files are gone");
    assert!(launched(&ids[0]), "the other's stay");
}

#[test]
fn force_delete_takes_a_session_with_unpushed_and_uncommitted_work() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
    let path = std::path::PathBuf::from(fixture::worktree(&mut state, &services, &spawner));
    std::fs::write(path.join("b.txt"), "committed\n").unwrap();
    fixture::sh(&path, &["add", "."]);
    fixture::sh(&path, &["commit", "-m", "unpushed"]);
    std::fs::write(path.join("a.txt"), "dirty\n").unwrap();
    let id = state.session.selected.clone().expect("the session");

    let delete = Command::ForceDelete {
        session: id.clone(),
    };
    dispatch(Cmd::Session(delete), &mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| {
        s.session.living.iter().all(|one| one.session.id != id)
    });
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert!(state.session.get(&id).is_none(), "off the rail");
    assert!(!path.exists(), "the worktree is gone, changes and all");
}

#[test]
fn a_refused_delete_leaves_the_session_its_agent_and_its_files() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
    let path = std::path::PathBuf::from(fixture::worktree(&mut state, &services, &spawner));
    std::fs::write(path.join("a.txt"), "dirty\n").unwrap();
    let id = state.session.selected.clone().expect("the session");
    until(&spawner, &services, &mut state, |s| {
        !s.session.feed.is_empty()
    });
    let prompt = home
        .path()
        .join(format!("data/agent-launch/{id}.prompt.md"));
    assert!(prompt.is_file(), "the launch wrote its files");

    let delete = Command::DeleteLocal {
        session: id.clone(),
    };
    dispatch(Cmd::Session(delete), &mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| !s.errors.is_empty());
    until(&spawner, &services, &mut state, |s| {
        s.agent.agent(&id).is_some()
    });
    assert!(state.session.get(&id).is_some(), "still on the rail");
    assert!(path.exists(), "its worktree stays");
    assert!(prompt.is_file(), "its launch files stay");
    let lines = &state.session.feed;
    assert!(lines.iter().any(|one| one.session == id), "its lines stay");
}
