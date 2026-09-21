//! What a worktree's MR makes the task it belongs to ask for.

use groove_types::{
    Attention, CiState, ExternalId, MrFacts, MrState, ProviderId, SessionKind, Task, TaskDates,
    Timestamp, WorktreeId,
};

use super::*;

const DAY: i64 = 86_400;

fn external() -> ExternalId {
    ExternalId::new("github.com/haoov/groove#50")
}

fn task() -> Task {
    Task {
        external_id: external(),
        short_id: "gh-haoov-groove-50".into(),
        title: "Harden Groove".into(),
        status: "In progress".into(),
        intent: None,
        priority: None,
        dates: TaskDates::default(),
        estimate: None,
        logged: None,
        synced_at: Timestamp::now(),
        provider: ProviderId::Github,
        url: None,
        board: None,
        branch_tag: None,
    }
}

/// The fixture's worktree, its session working a task the slice knows.
fn working(state: &mut crate::AppState, services: &Services, spawner: &SyncSpawner) -> WorktreeId {
    worktree(state, services, spawner);
    let id = state.session.selected.clone().expect("a session");
    let open = state.session.get_mut(&id).expect("the open session");
    open.session.kind = SessionKind::Task {
        external_id: external(),
    };
    let worktree = open
        .selected_worktree()
        .map(|one| one.id.clone())
        .expect("a worktree");
    state.task.tasks = vec![task()];
    worktree
}

fn now() -> Timestamp {
    Timestamp::new(20 * DAY)
}

#[test]
fn a_review_nobody_has_answered_wakes_the_task() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let worktree = working(&mut state, &services, &spawner);
    assert!(!state.task.asks(&external()), "nothing asks yet");

    state.workspace.facts.insert(
        worktree,
        MrFacts {
            state: Some(MrState::Open),
            review_requested_at: Some(Timestamp::new(16 * DAY)),
            ..Default::default()
        },
    );
    crate::task::attention::reread(&mut state, now());
    assert_eq!(
        state.task.needs(&external()),
        [Attention::ReviewWaiting {
            since: Timestamp::new(16 * DAY)
        }]
    );
}

#[test]
fn a_run_that_failed_and_changes_asked_for_both_wake_it() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let worktree = working(&mut state, &services, &spawner);
    state.workspace.facts.insert(
        worktree,
        MrFacts {
            state: Some(MrState::Open),
            changes_requested_at: Some(Timestamp::new(18 * DAY)),
            ci: Some(CiState::Failed),
            ci_finished_at: Some(Timestamp::new(19 * DAY)),
            ..Default::default()
        },
    );
    crate::task::attention::reread(&mut state, now());
    assert_eq!(
        state.task.needs(&external()),
        [
            Attention::ChangesRequested {
                since: Timestamp::new(18 * DAY)
            },
            Attention::CiFailed {
                since: Some(Timestamp::new(19 * DAY))
            }
        ]
    );
}

#[test]
fn an_mr_approved_and_green_but_unmerged_wakes_it() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let worktree = working(&mut state, &services, &spawner);
    state.workspace.facts.insert(
        worktree.clone(),
        MrFacts {
            state: Some(MrState::Open),
            approved_at: Some(Timestamp::new(17 * DAY)),
            ci: Some(CiState::Success),
            ..Default::default()
        },
    );
    crate::task::attention::reread(&mut state, now());
    assert_eq!(
        state.task.needs(&external()),
        [Attention::ApprovedUnmerged {
            since: Timestamp::new(17 * DAY)
        }]
    );

    state.workspace.facts.remove(&worktree);
    crate::task::attention::reread(&mut state, now());
    assert!(
        !state.task.asks(&external()),
        "with no mr of its own it asks nothing"
    );
}

#[test]
fn a_merged_mr_leaves_the_task_alone() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let worktree = working(&mut state, &services, &spawner);
    state.workspace.facts.insert(
        worktree,
        MrFacts {
            state: Some(MrState::Merged),
            approved_at: Some(Timestamp::new(10 * DAY)),
            ci: Some(CiState::Success),
            ..Default::default()
        },
    );
    crate::task::attention::reread(&mut state, now());
    assert!(!state.task.asks(&external()));
}
