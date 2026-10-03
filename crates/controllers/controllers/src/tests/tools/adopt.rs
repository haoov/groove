//! An explorer made the session of a task its agent filed at the source.

use groove_agent_service::{Answer, Call, Reply};
use groove_types::{SessionId, SessionKind};
use serde_json::json;

use super::settled;
use crate::tests::fixture::{self, services, state, worktree};
use crate::{AppState, Services, SyncSpawner};

const ISSUE: &str = "https://github.com/haoov/groove/issues/50";

/// The explorer, its writes let through, beside a GitHub source that answers.
struct Exploring {
    _runtime: tokio::runtime::Runtime,
    _server: wiremock::MockServer,
    state: AppState,
    services: Services,
    spawner: SyncSpawner,
    id: SessionId,
}

fn exploring(home: &std::path::Path) -> Exploring {
    let (runtime, server) = crate::tests::tasks::answering();
    fixture::pooled_clone(home);
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home);
    let mut state = state(home);
    let host = format!("http://{}", server.address());
    state.config.config.as_mut().expect("a config").github =
        Some(serde_json::from_value(crate::tests::tasks::source(&host)).expect("the source"));
    worktree(&mut state, &services, &spawner);
    let id = state.session.selected.clone().expect("the explorer");
    let on = crate::agent::Command::AutoApprove {
        session: id.clone(),
        on: true,
    };
    crate::dispatch(crate::Command::Agent(on), &mut state, &services, &spawner);
    Exploring {
        _runtime: runtime,
        _server: server,
        state,
        services,
        spawner,
        id,
    }
}

/// The tool called, and its answer once the read and the write behind it have landed.
fn adopted(at: &mut Exploring, task: &str) -> Answer {
    let from = at.id.clone();
    adopted_from(at, &from, task)
}

fn adopted_from(at: &mut Exploring, from: &SessionId, task: &str) -> Answer {
    let (reply, mut answered) = Reply::new();
    let call = Call {
        session: from.as_str().to_string(),
        tool: "adopt_task".into(),
        arguments: json!({ "task": task }),
        reply,
    };
    crate::tools::answer(&mut at.state, &at.services, &at.spawner, call);
    settled(&at.spawner, &at.services, &mut at.state, &mut answered)
}

const TASK: &str = "gh-haoov-groove-50";

/// The answer, then the promotion that follows it.
fn promoted(at: &mut Exploring) -> Answer {
    let answer = adopted(at, ISSUE);
    let task = SessionId::new(TASK);
    crate::tests::fixture::until(&at.spawner, &at.services, &mut at.state, |s| {
        s.session.get(&task).is_some()
            && s.agent.activity(&task).is_some()
            && s.session
                .get(&task)
                .is_some_and(|one| one.state.auto_approve)
    });
    answer
}

#[test]
fn an_explorer_is_promoted_to_the_task_s_own_session_and_is_gone() {
    let home = tempfile::tempdir().unwrap();
    let mut at = exploring(home.path());
    let explorer = at.id.clone();
    let before = at.state.session.get(&explorer).expect("open").worktrees[0].clone();

    let answer = promoted(&mut at);
    assert!(!answer.failed, "{}", answer.text);
    assert!(answer.text.contains(TASK), "{}", answer.text);
    assert!(
        at.state.session.get(&explorer).is_none(),
        "the explorer is gone"
    );
    let task = SessionId::new(TASK);
    assert_eq!(at.state.session.selected.as_ref(), Some(&task));

    let open = at.state.session.get(&task).expect("the task's own session");
    assert!(matches!(open.session.kind, SessionKind::Task { .. }));
    assert_eq!(open.session.title, "Harden Groove");
    let after = &open.worktrees[0];
    assert!(!after.branch.starts_with("explorer/"), "{}", after.branch);
    assert!(
        after.path.contains(&format!("/worktrees/{TASK}/")),
        "{}",
        after.path
    );
    assert!(
        !std::path::Path::new(&before.path).exists(),
        "the old path is gone"
    );
    let branch = crate::tests::fixture::sh(
        std::path::Path::new(&after.path),
        &["rev-parse", "--abbrev-ref", "HEAD"],
    );
    assert_eq!(branch.trim(), after.branch, "git agrees");

    let store = at.services.session.store();
    let row = |id: &SessionId| at.spawner.block_on(store.get(id)).expect("a read");
    assert!(row(&explorer).is_none(), "the explorer's row is gone");
    assert!(matches!(
        row(&task).expect("the task's row").kind,
        SessionKind::Task { .. }
    ));
    let handed = home
        .path()
        .join("data/agent-launch")
        .join(format!("{TASK}.thread"));
    assert!(
        std::fs::read_to_string(&handed).is_ok_and(|uuid| !uuid.trim().is_empty()),
        "the task's agent carries on the explorer's conversation: {}",
        handed.display()
    );
}

#[test]
fn a_session_that_is_not_an_explorer_is_refused() {
    let home = tempfile::tempdir().unwrap();
    let mut at = exploring(home.path());
    let first = promoted(&mut at);
    assert!(!first.failed, "{}", first.text);

    let again = adopted_from(&mut at, &SessionId::new(TASK), ISSUE);
    assert!(again.failed, "{}", again.text);
    assert!(again.text.contains("explorer"), "{}", again.text);
}

#[test]
fn a_reference_that_names_no_task_is_refused_before_anything_is_read() {
    let home = tempfile::tempdir().unwrap();
    let mut at = exploring(home.path());
    let answer = adopted(&mut at, "the auth refresh task");
    assert!(answer.failed, "{}", answer.text);
    let open = at.state.session.get(&at.id).expect("open");
    assert!(matches!(open.session.kind, SessionKind::Explorer));
}

#[test]
fn a_task_another_session_already_works_is_refused() {
    let home = tempfile::tempdir().unwrap();
    let mut at = exploring(home.path());
    let explorer = at.id.clone();
    for command in [
        crate::task::Command::Load,
        crate::task::Command::Open {
            short_id: "gh-haoov-groove-50".into(),
        },
    ] {
        crate::dispatch(
            crate::Command::Task(command),
            &mut at.state,
            &at.services,
            &at.spawner,
        );
        crate::tests::fixture::until(&at.spawner, &at.services, &mut at.state, |s| {
            !s.task.tasks.is_empty()
        });
    }
    assert!(
        at.state.session.open.len() >= 2,
        "the task has a session of its own"
    );

    let answer = adopted(&mut at, ISSUE);
    assert!(answer.failed, "{}", answer.text);
    assert!(answer.text.contains("already works"), "{}", answer.text);
    let open = at.state.session.get(&explorer).expect("open");
    assert!(matches!(open.session.kind, SessionKind::Explorer));
}

#[test]
fn a_promotion_that_fails_puts_every_worktree_back_and_leaves_the_explorer() {
    let home = tempfile::tempdir().unwrap();
    let mut at = exploring(home.path());
    let explorer = at.id.clone();
    let before = at.state.session.get(&explorer).expect("open").worktrees[0].clone();
    let taken = groove_types::Session {
        id: SessionId::new(TASK),
        title: "an earlier session of the task".into(),
        kind: SessionKind::Task {
            external_id: groove_types::ExternalId::new("elsewhere"),
        },
        created_at: groove_types::Timestamp::new(0),
    };
    at.spawner
        .block_on(
            at.services
                .session
                .store()
                .create_explorer(&groove_types::Session {
                    kind: SessionKind::Explorer,
                    ..taken
                }),
        )
        .expect("a row already holds the task's id");

    let answer = adopted(&mut at, ISSUE);
    assert!(!answer.failed, "{}", answer.text);
    crate::tests::fixture::until(&at.spawner, &at.services, &mut at.state, |s| {
        !s.errors.is_empty()
    });

    let open = at.state.session.get(&explorer).expect("the explorer stays");
    assert!(matches!(open.session.kind, SessionKind::Explorer));
    assert_eq!(open.worktrees[0], before, "its worktree as it was");
    assert!(
        std::path::Path::new(&before.path).exists(),
        "back where it was"
    );
    let branch = crate::tests::fixture::sh(
        std::path::Path::new(&before.path),
        &["rev-parse", "--abbrev-ref", "HEAD"],
    );
    assert_eq!(branch.trim(), before.branch, "on its own branch again");
}

#[test]
fn the_explorer_leaves_no_launch_file_and_no_feed_line_behind() {
    let home = tempfile::tempdir().unwrap();
    let mut at = exploring(home.path());
    let explorer = at.id.clone();
    let launch = home.path().join("data/agent-launch");
    assert!(launch.join(format!("{explorer}.prompt.md")).is_file());

    let answer = promoted(&mut at);
    assert!(!answer.failed, "{}", answer.text);
    let left: Vec<String> = std::fs::read_dir(&launch)
        .unwrap()
        .map(|one| one.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(explorer.as_str()))
        .collect();
    assert!(left.is_empty(), "{left:?}");
    assert!(
        at.state
            .session
            .feed
            .iter()
            .all(|line| line.session != explorer),
        "{:?}",
        at.state.session.feed
    );
}
