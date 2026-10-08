use groove_types::{KubeAuth, KubeContext, Login};

use crate::{Event, State, apply};

fn context(name: &str) -> KubeContext {
    KubeContext {
        name: name.into(),
        cluster: name.into(),
        server: None,
        namespace: None,
        auth: KubeAuth::Token,
    }
}

#[test]
fn a_second_scan_waits_for_the_first() {
    let mut state = State::default();
    assert!(state.begin_scan());
    assert!(!state.begin_scan());
    apply(&mut state, Event::Found(vec![context("kind")]));
    assert!(state.begin_scan());
}

#[test]
fn a_scan_that_cannot_read_keeps_what_the_last_one_found() {
    let mut state = State::default();
    apply(&mut state, Event::Found(vec![context("kind")]));
    state.begin_scan();
    apply(&mut state, Event::Unread);
    assert_eq!(state.found, Some(vec![context("kind")]));
    assert!(!state.scanning);
}

#[test]
fn a_check_replaces_the_last_login_of_its_context_only() {
    let mut state = State::default();
    let refused = Login::Refused("token expired".into());
    let signed_in = Login::SignedIn {
        version: "v1.33.4".into(),
    };
    apply(
        &mut state,
        Event::Checked {
            context: "hub".into(),
            login: refused,
        },
    );
    assert!(state.begin_check("hub"));
    assert!(!state.begin_check("hub"));
    apply(
        &mut state,
        Event::Checked {
            context: "hub".into(),
            login: signed_in.clone(),
        },
    );
    assert_eq!(state.login("hub"), Some(&signed_in));
    assert_eq!(state.login("elpis"), None);
    assert!(!state.checking("hub"));
}

fn pods(namespace: &str) -> groove_types::WatchKey {
    groove_types::WatchKey {
        context: "kind".into(),
        kind: groove_types::KubeKind {
            group: String::new(),
            version: "v1".into(),
            kind: "Pod".into(),
            plural: "pods".into(),
            namespaced: true,
            watchable: true,
        },
        namespace: Some(namespace.into()),
        selector: None,
    }
}

fn row(uid: &str, name: &str, status: &str) -> groove_types::ObjectRow {
    groove_types::ObjectRow {
        uid: uid.into(),
        name: name.into(),
        namespace: Some("paxone".into()),
        version: "1".into(),
        cells: vec![name.into(), status.into()],
        aging: Vec::new(),
    }
}

#[test]
fn a_watcher_starts_for_its_first_reader_and_is_unread_once_its_last_lets_go() {
    let mut store = crate::Store::default();
    let key = pods("paxone");
    assert!(
        store.lease(&key, "list").is_some(),
        "the first reader starts it"
    );
    assert!(store.lease(&key, "tab").is_none());
    assert!(store.release("list").is_empty(), "the tab reads it still");
    assert_eq!(store.release("tab"), std::slice::from_ref(&key));
    store.lease(&key, "list");
    store.drop_unread(&key);
    assert!(
        store.watched(&key).is_some(),
        "read again before it was dropped"
    );
    store.release("list");
    store.drop_unread(&key);
    assert!(store.watched(&key).is_none());
}

#[test]
fn rows_stand_by_name_and_each_change_lands_in_its_place() {
    use crate::{Batch, Delta};
    let mut store = crate::Store::default();
    let key = pods("paxone");
    store.lease(&key, "list");
    let rows = vec![row("b", "worker", "Running"), row("a", "api", "Running")];
    store.apply(
        &key,
        Batch::Reset {
            columns: Vec::new(),
            rows,
        },
    );
    let changes = vec![
        Delta::Put(row("c", "front", "Pending")),
        Delta::Put(row("b", "worker", "CrashLoopBackOff")),
        Delta::Gone(row("a", "api", "Running")),
    ];
    store.apply(&key, Batch::Changes(changes));
    let held = store.watched(&key).expect("watched");
    let shown: Vec<(&str, &str)> = held
        .rows
        .iter()
        .map(|one| (one.name.as_str(), one.cells[1].as_str()))
        .collect();
    assert_eq!(
        shown,
        [("front", "Pending"), ("worker", "CrashLoopBackOff")]
    );
    assert!(held.synced);
    store.apply(&key, Batch::Failed("unreachable".into()));
    assert_eq!(
        store.watched(&key).and_then(|one| one.failed.as_deref()),
        Some("unreachable")
    );
    store.release("list");
    store.drop_unread(&key);
    store.apply(
        &key,
        Batch::Changes(vec![Delta::Gone(row("c", "front", "Pending"))]),
    );
    assert!(
        store.watched(&key).is_none(),
        "a batch of a dropped watcher is ignored"
    );
}

#[test]
fn a_row_made_again_under_its_name_outlives_the_late_gone_of_the_one_before() {
    use crate::{Batch, Delta};
    let mut store = crate::Store::default();
    let key = pods("paxone");
    store.lease(&key, "list");
    store.apply(
        &key,
        Batch::Reset {
            columns: Vec::new(),
            rows: vec![row("old", "api", "Running")],
        },
    );
    let changes = vec![
        Delta::Put(row("new", "api", "Pending")),
        Delta::Gone(row("old", "api", "Running")),
    ];
    store.apply(&key, Batch::Changes(changes));
    let held = store.watched(&key).expect("watched");
    let uids: Vec<&str> = held.rows.iter().map(|one| one.uid.as_str()).collect();
    assert_eq!(uids, ["new"]);
}

/// Run with `cargo test --release -p groove-cluster-service time -- --ignored --nocapture`.
#[test]
#[ignore]
#[allow(clippy::print_stdout)]
fn time_a_reset_of_30k_rows_then_batches_of_500_changes_and_an_age_tick() {
    use crate::{Batch, Delta};
    use groove_types::{Aging, Timestamp};
    use std::time::Instant;
    let named = |at: usize| groove_types::ObjectRow {
        aging: vec![Aging {
            cell: 1,
            since: Timestamp::new(1_000 + at as i64 % 600),
            said: 0,
            lead: None,
            turn: Timestamp::default(),
        }],
        ..row(
            &format!("u{at}"),
            &format!("api-{at:05}-7d9f8c6b5-x2k4p"),
            "Running",
        )
    };
    let mut store = crate::Store::default();
    let key = pods("paxone");
    store.lease(&key, "list");
    let rows: Vec<_> = (0..30_000).rev().map(named).collect();
    let started = Instant::now();
    store.apply(
        &key,
        Batch::Reset {
            columns: Vec::new(),
            rows,
        },
    );
    println!("reset of 30k rows: {:?}", started.elapsed());
    for run in 0..5 {
        let fresh = 30_000 + run * 200;
        let mut changes: Vec<Delta> = (0..200).map(|at| Delta::Put(named(at * 37))).collect();
        changes.extend((fresh..fresh + 150).map(|at| Delta::Put(named(at))));
        changes.extend((0..150).map(|at| Delta::Gone(named(run * 1000 + at * 3 + 1))));
        let started = Instant::now();
        store.apply(&key, Batch::Changes(changes));
        println!("batch of 500 changes: {:?}", started.elapsed());
    }
    for now in [1_700, 1_701] {
        let started = Instant::now();
        store.age(Timestamp::new(now));
        println!("age tick at {now}: {:?}", started.elapsed());
    }
}

#[test]
fn a_failure_outlasts_a_relist_and_ends_once_a_watch_is_open() {
    use crate::Batch;
    let mut store = crate::Store::default();
    let key = pods("paxone");
    store.lease(&key, "list");
    store.apply(&key, Batch::Failed("watch refused".into()));
    store.apply(
        &key,
        Batch::Reset {
            columns: Vec::new(),
            rows: vec![row("a", "api", "Running")],
        },
    );
    assert_eq!(
        store.watched(&key).and_then(|one| one.failed.as_deref()),
        Some("watch refused")
    );
    store.apply(&key, Batch::Watching);
    assert_eq!(
        store.watched(&key).and_then(|one| one.failed.as_deref()),
        None
    );
}

#[test]
fn a_row_with_an_age_is_due_at_once_and_each_tick_writes_it_and_waits_for_its_next_turn() {
    use crate::{Batch, Delta};
    use groove_types::{Aging, Timestamp};
    let mut store = crate::Store::default();
    let key = pods("paxone");
    store.lease(&key, "list");
    let aged = |name: &str| groove_types::ObjectRow {
        aging: vec![Aging {
            cell: 1,
            since: Timestamp::new(1_000),
            said: 0,
            lead: None,
            turn: Timestamp::default(),
        }],
        ..row(name, name, "40s")
    };
    let rows = vec![aged("api")];
    store.apply(
        &key,
        Batch::Reset {
            columns: Vec::new(),
            rows,
        },
    );
    assert_eq!(store.due(), Some(Timestamp::default()));
    store.age(Timestamp::new(1_179));
    assert_eq!(
        store.watched(&key).expect("watched").rows[0].cells[1],
        "2m59s"
    );
    assert_eq!(store.due(), Some(Timestamp::new(1_180)));
    store.apply(&key, Batch::Changes(vec![Delta::Put(aged("worker"))]));
    assert_eq!(
        store.due(),
        Some(Timestamp::default()),
        "a new row is written at once"
    );
    store.age(Timestamp::new(1_180));
    let cells: Vec<&str> = store
        .watched(&key)
        .expect("watched")
        .rows
        .iter()
        .map(|one| one.cells[1].as_str())
        .collect();
    assert_eq!(cells, ["3m", "3m"]);
}
