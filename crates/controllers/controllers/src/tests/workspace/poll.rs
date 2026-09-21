//! Which worktrees a tick reads: focus, the interval, and what is already out.

use groove_types::{MrDelivery, MrState, Timestamp, WorktreeId};

use super::*;
use crate::event::{Event, Window, apply};
use crate::workspace::mr::{INTERVAL, wanted};

/// The fixture's worktree, focused, on a host whose forge Groove reads.
fn working(state: &mut crate::AppState, services: &Services, spawner: &SyncSpawner) -> WorktreeId {
    worktree(state, services, spawner);
    state.focused = true;
    host(state, "github.com");
    selected(state)
}

/// The host the session's repo stands on.
fn host(state: &mut crate::AppState, host: &str) {
    let id = selected_session(state);
    let open = state.session.get_mut(&id).expect("the open session");
    for repo in open.repos.iter_mut() {
        repo.host = host.to_string();
    }
}

fn selected(state: &crate::AppState) -> WorktreeId {
    state
        .session
        .selected()
        .and_then(|open| open.selected_worktree())
        .map(|one| one.id.clone())
        .expect("a selected worktree")
}

/// Gives the worktree's row an MR in this state.
fn holds(state: &mut crate::AppState, worktree: &WorktreeId, mr: MrState) {
    let open = state
        .session
        .get_mut(&selected_session(state))
        .expect("the open session");
    open.row(worktree).mr = Some(MrDelivery {
        forge: groove_types::Forge::Github,
        number: "7".into(),
        state: mr,
        url: "https://example.com/mr/1".into(),
        approved: false,
        changes_requested: false,
    });
}

fn selected_session(state: &crate::AppState) -> groove_types::SessionId {
    state
        .session
        .selected()
        .map(|open| open.session.id.clone())
        .expect("a selected session")
}

fn now() -> Timestamp {
    Timestamp::new(1_800_000_000)
}

#[test]
fn a_window_nobody_is_looking_at_reads_nothing() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let id = working(&mut state, &services, &spawner);
    assert_eq!(wanted(&state, now()), [id], "focused, and never asked");

    state.focused = false;
    assert!(wanted(&state, now()).is_empty());
}

#[test]
fn the_selected_worktree_is_asked_about_once() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let id = working(&mut state, &services, &spawner);
    assert_eq!(wanted(&state, now()), vec![id.clone()]);

    state.workspace.poll.sent(&id);
    state.workspace.poll.answered(&id);
    assert!(
        wanted(&state, now()).is_empty(),
        "it has no mr, and is not asked again"
    );
}

#[test]
fn an_open_mr_is_read_again_when_the_interval_has_passed() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let id = working(&mut state, &services, &spawner);
    holds(&mut state, &id, MrState::Open);
    state.workspace.poll.sent(&id);
    state.workspace.poll.answered(&id);
    state.workspace.poll.ran(now());

    assert!(wanted(&state, now()).is_empty(), "the pass just ran");
    let later = Timestamp::new(now().seconds() + INTERVAL);
    assert_eq!(wanted(&state, later), [id], "the interval has passed");
}

#[test]
fn an_mr_that_is_no_longer_open_is_left_alone() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let id = working(&mut state, &services, &spawner);
    holds(&mut state, &id, MrState::Merged);
    state.workspace.poll.sent(&id);
    state.workspace.poll.answered(&id);
    state.workspace.poll.ran(now());

    let later = Timestamp::new(now().seconds() + INTERVAL);
    assert!(wanted(&state, later).is_empty());
}

#[test]
fn a_read_still_out_is_not_sent_a_second_time() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let id = working(&mut state, &services, &spawner);
    holds(&mut state, &id, MrState::Open);
    state.workspace.poll.sent(&id);
    state.workspace.poll.ran(now());

    let later = Timestamp::new(now().seconds() + INTERVAL);
    assert!(wanted(&state, later).is_empty(), "one read is already out");
    state.workspace.poll.answered(&id);
    assert_eq!(wanted(&state, later), [id]);
}

#[test]
fn the_window_coming_back_asks_about_everything_again() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let id = working(&mut state, &services, &spawner);
    state.workspace.poll.sent(&id);
    state.workspace.poll.answered(&id);
    state.workspace.poll.ran(now());
    assert!(wanted(&state, now()).is_empty());

    apply(Event::Window(Window::Focus(true)), &mut state);
    assert_eq!(wanted(&state, now()), [id], "asked about once on focus");
}

#[test]
fn a_gitlab_worktree_is_asked_about_like_any_other() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let id = working(&mut state, &services, &spawner);
    assert_eq!(wanted(&state, now()), vec![id.clone()]);

    host(&mut state, "gitlab.wiremind.io");
    assert_eq!(
        wanted(&state, now()),
        vec![id],
        "gitlab has a client of its own now"
    );
}
