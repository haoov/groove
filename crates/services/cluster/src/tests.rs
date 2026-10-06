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
    }
}

fn row(uid: &str, name: &str, status: &str) -> groove_types::ObjectRow {
    groove_types::ObjectRow {
        uid: uid.into(),
        name: name.into(),
        namespace: Some("paxone".into()),
        version: "1".into(),
        cells: vec![name.into(), status.into()],
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
        Delta::Gone("a".into()),
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
    store.apply(&key, Batch::Changes(vec![Delta::Gone("c".into())]));
    assert!(
        store.watched(&key).is_none(),
        "a batch of a dropped watcher is ignored"
    );
}
