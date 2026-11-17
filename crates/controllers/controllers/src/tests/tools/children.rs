//! A maintenance task spawns children, and a child logs to it before it closes.

use groove_agent_service::{Answer, Call, Reply};
use groove_types::{SessionId, Timestamp};
use serde_json::json;

use super::settled;
use crate::tests::fixture::{self, auto_approve, until};
use crate::{AppState, Command, Services, SyncSpawner, dispatch, session};

const PARENT: &str = "MAINT-47";

pub(crate) struct Maintaining {
    pub _home: tempfile::TempDir,
    pub state: AppState,
    pub services: Services,
    pub spawner: SyncSpawner,
    pub parent: SessionId,
}

/// A task on the rail, in the store, its writes let through; marked when `marked`.
pub(crate) fn maintaining(marked: bool) -> Maintaining {
    let (home, spawner, services, mut state) = fixture::fresh();
    let task = groove_types::Task {
        external_id: groove_types::ExternalId::new(format!("notion:{PARENT}")),
        short_id: PARENT.into(),
        title: "Maintenance 2026 week 47".into(),
        status: "In progress".into(),
        intent: None,
        priority: None,
        dates: groove_types::TaskDates::default(),
        estimate: None,
        logged: None,
        synced_at: Timestamp::now(),
        provider: groove_types::ProviderId::Notion,
        url: None,
        project: None,
        branch_tag: None,
    };
    let now = Timestamp::now();
    let session = groove_session_service::task_session(&task, now);
    spawner
        .block_on(services.session.create_task(&session, &task, now))
        .expect("the task's session");
    let parent = session.id.clone();
    state.session.open(session, now);
    auto_approve(&mut state, &parent);
    if marked {
        let mark = session::Command::SetMaintenance {
            session: parent.clone(),
            on: true,
        };
        dispatch(Command::Session(mark), &mut state, &services, &spawner);
    }
    Maintaining {
        _home: home,
        state,
        services,
        spawner,
        parent,
    }
}

fn called(
    at: &mut Maintaining,
    from: &SessionId,
    tool: &str,
    arguments: serde_json::Value,
) -> Answer {
    let (reply, mut answered) = Reply::new();
    let call = Call {
        session: from.as_str().to_string(),
        tool: tool.into(),
        arguments,
        reply,
    };
    crate::tools::answer(&mut at.state, &at.services, &at.spawner, call);
    settled(&at.spawner, &at.services, &mut at.state, &mut answered)
}

fn spawned(at: &mut Maintaining) -> Answer {
    let parent = at.parent.clone();
    let brief = json!({ "title": "VeleroBackupFailed", "brief": "Find why daily-full failed." });
    called(at, &parent, "spawn_session", brief)
}

#[test]
fn a_task_not_marked_as_maintenance_spawns_no_child() {
    let mut at = maintaining(false);
    let answer = spawned(&mut at);
    assert!(answer.failed, "{}", answer.text);
    assert_eq!(at.state.session.open.len(), 1, "no child on the rail");
}

#[test]
fn a_maintenance_task_spawns_a_child_beside_it_and_remembers_it() {
    let mut at = maintaining(true);
    let answer = spawned(&mut at);
    assert!(!answer.failed, "{}", answer.text);
    let parent = at.state.session.get(&at.parent).expect("the parent");
    let child = parent.children[0].clone();
    assert_eq!(child.title, "VeleroBackupFailed");
    assert_eq!(at.state.session.selected.as_ref(), Some(&at.parent));
    let open = at
        .state
        .session
        .get(&child.id)
        .expect("the child on the rail");
    assert_eq!(open.state.parent.as_ref(), Some(&at.parent));

    let store = at.services.session.store();
    until(&at.spawner, &at.services, &mut at.state, |_| {
        at.spawner.idle()
    });
    let state = at.spawner.block_on(store.state(&child.id)).expect("a read");
    assert_eq!(state.parent.as_ref(), Some(&at.parent));
    assert!(
        at.spawner
            .block_on(store.state(&at.parent))
            .expect("a read")
            .maintenance
    );
    let children = at
        .spawner
        .block_on(store.children(&at.parent))
        .expect("a read");
    assert_eq!(children, [child]);
}

#[test]
fn a_closed_child_hands_its_newest_entry_to_its_parent() {
    let mut at = maintaining(true);
    spawned(&mut at);
    let child = at
        .state
        .session
        .get(&at.parent)
        .expect("the parent")
        .children[0]
        .id
        .clone();
    auto_approve(&mut at.state, &child);
    let entry = json!({
        "what": "Pruned the snapshots older than the TTL.",
        "why": "The snapshot quota was full.",
    });
    let answer = called(
        &mut at,
        &child,
        "log_work",
        json!({ "what": "Read the job's logs." }),
    );
    assert!(!answer.failed, "{}", answer.text);
    let answer = called(&mut at, &child, "log_work", entry);
    assert!(!answer.failed, "{}", answer.text);
    let parent = at.parent.clone();
    let own = json!({ "what": "Spawned a child for the Velero alert." });
    let answer = called(&mut at, &parent, "log_work", own);
    assert!(!answer.failed, "{}", answer.text);
    let log = |at: &Maintaining| {
        at.state
            .session
            .get(&at.parent)
            .expect("the parent")
            .log
            .clone()
    };
    assert_eq!(log(&at).len(), 1, "the child keeps its log while it lives");
    let child_log = &at.state.session.get(&child).expect("the child").log;
    assert_eq!(child_log[0].from, None);

    session::delete(&mut at.state, &at.services, &at.spawner, &child, false);
    until(&at.spawner, &at.services, &mut at.state, |_| {
        at.spawner.idle()
    });
    let handed = log(&at);
    assert_eq!(handed.len(), 2, "one entry per child: {handed:?}");
    let from_child = handed
        .iter()
        .find(|one| one.from.is_some())
        .expect("handed");
    assert_eq!(from_child.from.as_deref(), Some("VeleroBackupFailed"));
    assert_eq!(
        from_child.why.as_deref(),
        Some("The snapshot quota was full.")
    );
    let store = at.services.session.store();
    let kept = at
        .spawner
        .block_on(store.log_of(&at.parent))
        .expect("a read");
    assert_eq!(kept, handed);
}

#[test]
fn only_a_task_is_marked_as_maintenance() {
    let (_home, spawner, services, mut state) = fixture::with_explorer();
    let explorer = SessionId::new("explorer-1");
    let mark = session::Command::SetMaintenance {
        session: explorer.clone(),
        on: true,
    };
    dispatch(Command::Session(mark), &mut state, &services, &spawner);
    assert!(
        !state
            .session
            .get(&explorer)
            .expect("open")
            .state
            .maintenance
    );
    assert!(!state.errors.is_empty(), "the user hears why");
}

#[test]
fn a_log_entry_removed_leaves_the_log_and_the_store() {
    let mut at = maintaining(true);
    let parent = at.parent.clone();
    for what in ["Rotated the token.", "Logged by mistake."] {
        let answer = called(&mut at, &parent, "log_work", json!({ "what": what }));
        assert!(!answer.failed, "{}", answer.text);
    }
    let log = &at.state.session.get(&parent).expect("the parent").log;
    let wrong = log[0].id;
    let remove = session::Command::RemoveLogEntry {
        session: parent.clone(),
        entry: wrong,
    };
    dispatch(
        Command::Session(remove),
        &mut at.state,
        &at.services,
        &at.spawner,
    );
    until(&at.spawner, &at.services, &mut at.state, |_| {
        at.spawner.idle()
    });
    let log = at
        .state
        .session
        .get(&parent)
        .expect("the parent")
        .log
        .clone();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].what, "Rotated the token.");
    let store = at.services.session.store();
    let kept = at.spawner.block_on(store.log_of(&parent)).expect("a read");
    assert_eq!(kept, log);
}
