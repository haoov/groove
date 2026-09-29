use groove_types::{RepoId, Session, SessionId, SessionKind, Timestamp};

use crate::{Error, Store};

fn explorer(id: &str, at: i64) -> Session {
    Session {
        id: SessionId::new(id),
        title: format!("explorer {id}"),
        kind: SessionKind::Explorer,
        created_at: Timestamp::new(at),
    }
}

#[tokio::test]
async fn an_explorer_round_trips_and_renames() {
    let store = Store::in_memory().await.unwrap();
    let session = explorer("explorer-1", 10);
    store.create_explorer(&session).await.unwrap();
    assert_eq!(store.get(&session.id).await.unwrap(), Some(session.clone()));
    store.rename_explorer(&session.id, "renamed").await.unwrap();
    assert_eq!(
        store.get(&session.id).await.unwrap().unwrap().title,
        "renamed"
    );
    let missing = store
        .rename_explorer(&SessionId::new("nope"), "x")
        .await
        .unwrap_err();
    assert!(matches!(
        missing,
        Error::NotFound {
            what: "explorer",
            ..
        }
    ));
    assert!(store.get(&SessionId::new("nope")).await.unwrap().is_none());
}

#[tokio::test]
async fn only_an_explorer_goes_through_create_explorer() {
    let store = Store::in_memory().await.unwrap();
    let task = Session {
        kind: SessionKind::Task {
            external_id: groove_types::ExternalId::new("page"),
        },
        ..explorer("t-1", 0)
    };
    assert!(matches!(
        store.create_explorer(&task).await,
        Err(Error::NotExplorer(_))
    ));
}

#[tokio::test]
async fn the_state_leaf_defaults_upserts_and_lists_the_opened_in_order() {
    let store = Store::in_memory().await.unwrap();
    for (id, at) in [("a", 1), ("b", 2), ("c", 3)] {
        store.create_explorer(&explorer(id, at)).await.unwrap();
    }
    let a = SessionId::new("a");
    assert_eq!(store.state(&a).await.unwrap(), Default::default());

    store
        .set_opened(&SessionId::new("c"), Some(Timestamp::new(100)))
        .await
        .unwrap();
    store
        .set_opened(&a, Some(Timestamp::new(200)))
        .await
        .unwrap();
    store.set_seen(&a, Timestamp::new(201)).await.unwrap();
    store.set_auto_approve(&a, true).await.unwrap();
    let state = store.state(&a).await.unwrap();
    assert_eq!(state.opened_at, Some(Timestamp::new(200)));
    assert_eq!(state.seen_at, Some(Timestamp::new(201)));
    assert!(state.auto_approve);

    let opened: Vec<String> = store
        .opened()
        .await
        .unwrap()
        .into_iter()
        .map(|(s, _)| s.id.to_string())
        .collect();
    assert_eq!(opened, ["c", "a"], "in the order opened");

    store.set_opened(&SessionId::new("c"), None).await.unwrap();
    let opened: Vec<String> = store
        .opened()
        .await
        .unwrap()
        .into_iter()
        .map(|(s, _)| s.id.to_string())
        .collect();
    assert_eq!(opened, ["a"]);
}

#[tokio::test]
async fn remove_cascades_to_the_leaf_and_the_repos() {
    let store = Store::in_memory().await.unwrap();
    let a = SessionId::new("a");
    store.create_explorer(&explorer("a", 1)).await.unwrap();
    store.set_opened(&a, Some(Timestamp::new(1))).await.unwrap();
    sqlx::query("INSERT INTO repos (id, host, group_path, project, local_path) VALUES ('r', 'h', 'g', 'p', '/p')")
        .execute(store.db.pool())
        .await
        .unwrap();
    store
        .attach_repo(&a, &RepoId::new("r"), Timestamp::new(2))
        .await
        .unwrap();
    assert_eq!(store.repos_of(&a).await.unwrap(), [RepoId::new("r")]);

    store.remove(&a).await.unwrap();
    assert!(store.get(&a).await.unwrap().is_none());
    assert_eq!(store.state(&a).await.unwrap(), Default::default());
    assert!(store.repos_of(&a).await.unwrap().is_empty());
    assert!(store.opened().await.unwrap().is_empty());
}

fn task() -> groove_types::Task {
    groove_types::Task {
        external_id: groove_types::ExternalId::new("github.com/haoov/groove#50"),
        short_id: "gh-haoov-groove-50".into(),
        title: "Harden Groove".into(),
        status: "In progress".into(),
        intent: None,
        priority: None,
        dates: groove_types::TaskDates::default(),
        estimate: None,
        logged: None,
        synced_at: Timestamp::new(0),
        provider: groove_types::ProviderId::Github,
        url: None,
        project: None,
        branch_tag: None,
    }
}

fn promoted() -> Session {
    Session {
        id: SessionId::new("gh-haoov-groove-50"),
        title: "Harden Groove".into(),
        kind: SessionKind::Task {
            external_id: groove_types::ExternalId::new("github.com/haoov/groove#50"),
        },
        created_at: Timestamp::new(20),
    }
}

#[tokio::test]
async fn an_explorer_promoted_hands_what_it_held_to_the_task_and_is_gone() {
    let store = Store::in_memory().await.unwrap();
    let explorer = explorer("explorer-1", 10);
    store.create_explorer(&explorer).await.unwrap();
    store.set_auto_approve(&explorer.id, true).await.unwrap();

    let session = promoted();
    store
        .promote(&explorer.id, &session, &task(), &[])
        .await
        .unwrap();
    assert!(
        store.get(&explorer.id).await.unwrap().is_none(),
        "the explorer is gone"
    );
    assert_eq!(store.get(&session.id).await.unwrap(), Some(session.clone()));
    assert!(
        store.state(&session.id).await.unwrap().auto_approve,
        "what the explorer held is the task's now"
    );
}

#[tokio::test]
async fn only_an_explorer_is_promoted_and_a_refusal_leaves_nothing_behind() {
    let store = Store::in_memory().await.unwrap();
    let session = promoted();
    let refused = store
        .promote(&SessionId::new("nope"), &session, &task(), &[])
        .await;
    assert!(matches!(refused, Err(Error::NotExplorer(_))));
    assert!(
        store.get(&session.id).await.unwrap().is_none(),
        "no row was left"
    );
}
