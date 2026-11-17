use crate::Stored as _;
use groove_types::{LogEntry, Timestamp};

use super::store::{explorer, promoted, task};
use crate::Store;

fn entry(what: &str, at: i64) -> LogEntry {
    LogEntry {
        id: 0,
        from: Some("KubeJobFailed".into()),
        what: what.into(),
        why: Some("the backup job ran out of memory".into()),
        mr_url: Some("https://github.com/acme/ops/pull/812".into()),
        at: Timestamp::new(at),
    }
}

#[tokio::test]
async fn a_task_marked_as_maintenance_lists_the_children_it_spawned() {
    let store = Store::in_memory().await.unwrap();
    let parent = promoted();
    store.create_task(&parent, &task()).await.unwrap();
    store.set_maintenance(&parent.id, true).await.unwrap();
    for (id, at) in [("explorer-b", 2), ("explorer-a", 1), ("explorer-c", 3)] {
        store.create_explorer(&explorer(id, at)).await.unwrap();
    }
    for id in ["explorer-b", "explorer-a"] {
        let child = groove_types::SessionId::new(id);
        store.set_parent(&child, &parent.id).await.unwrap();
    }

    assert!(store.state(&parent.id).await.unwrap().maintenance);
    let children: Vec<String> = store
        .children(&parent.id)
        .await
        .unwrap()
        .into_iter()
        .map(|one| one.id.to_string())
        .collect();
    assert_eq!(children, ["explorer-a", "explorer-b"], "oldest first");
    let state = store
        .state(&groove_types::SessionId::new("explorer-a"))
        .await
        .unwrap();
    assert_eq!(state.parent, Some(parent.id.clone()));
}

#[tokio::test]
async fn a_child_adopted_as_a_task_keeps_its_parent() {
    let store = Store::in_memory().await.unwrap();
    let parent = explorer("explorer-parent", 1);
    store.create_explorer(&parent).await.unwrap();
    let child = explorer("explorer-child", 2);
    store.create_explorer(&child).await.unwrap();
    store.set_parent(&child.id, &parent.id).await.unwrap();

    let adopted = promoted();
    store
        .promote(&child.id, &adopted, &task(), &[])
        .await
        .unwrap();
    let state = store.state(&adopted.id).await.unwrap();
    assert_eq!(state.parent, Some(parent.id.clone()));
}

#[tokio::test]
async fn a_deleted_parent_leaves_its_children_without_one() {
    let store = Store::in_memory().await.unwrap();
    let parent = explorer("explorer-parent", 1);
    store.create_explorer(&parent).await.unwrap();
    let child = explorer("explorer-child", 2);
    store.create_explorer(&child).await.unwrap();
    store.set_parent(&child.id, &parent.id).await.unwrap();

    store.remove(&parent.id).await.unwrap();
    assert!(store.get(&child.id).await.unwrap().is_some());
    assert_eq!(store.state(&child.id).await.unwrap().parent, None);
}

#[tokio::test]
async fn the_log_reads_back_newest_first_drops_an_entry_and_goes_with_its_session() {
    let store = Store::in_memory().await.unwrap();
    let parent = promoted();
    store.create_task(&parent, &task()).await.unwrap();
    let mut raised = entry("Raised the job's limit to 2Gi", 10);
    raised.id = store.append_log(&parent.id, &raised).await.unwrap();
    let mut rotated = entry("Rotated the token", 20);
    rotated.id = store.append_log(&parent.id, &rotated).await.unwrap();

    let log = store.log_of(&parent.id).await.unwrap();
    assert_eq!(log, [rotated.clone(), raised]);
    store.remove_log(&parent.id, log[1].id).await.unwrap();
    assert_eq!(store.log_of(&parent.id).await.unwrap(), [rotated]);
    store.remove(&parent.id).await.unwrap();
    assert!(store.log_of(&parent.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_task_remembers_one_child_per_alert() {
    let store = Store::in_memory().await.unwrap();
    let parent = promoted();
    store.create_task(&parent, &task()).await.unwrap();
    let child = explorer("explorer-child", 2);
    store.create_explorer(&child).await.unwrap();
    let alerted = groove_types::AlertChild {
        session: child.id.clone(),
        parent: parent.id.clone(),
        dedup_key: "key-velero".into(),
        title: "VeleroBackupFailed".into(),
        priority: Some("Critical".into()),
        cluster: Some("prod-eu".into()),
        namespace: Some("velero".into()),
    };
    store.remember_alert(&alerted).await.unwrap();
    let kept = store.alert_children(&parent.id).await.unwrap();
    assert_eq!(kept, std::slice::from_ref(&alerted));
    assert!(
        store.remember_alert(&alerted).await.is_err(),
        "one child per alert"
    );
    store.remove(&child.id).await.unwrap();
    assert!(store.alert_children(&parent.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_removed_child_hands_its_newest_entry_to_its_parent() {
    let store = Store::in_memory().await.unwrap();
    let parent = promoted();
    store.create_task(&parent, &task()).await.unwrap();
    let child = explorer("explorer-child", 2);
    store.create_explorer(&child).await.unwrap();
    store.set_parent(&child.id, &parent.id).await.unwrap();
    let mut own = entry("Pruned the snapshots", 10);
    own.from = None;
    store.append_log(&child.id, &own).await.unwrap();
    let mut first = entry("Read the job's logs", 5);
    first.from = None;
    store.append_log(&child.id, &first).await.unwrap();

    store.hand_log_over(&child.id).await.unwrap();
    store.remove(&child.id).await.unwrap();
    let log = store.log_of(&parent.id).await.unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].what, "Pruned the snapshots");
    assert_eq!(log[0].from.as_deref(), Some(child.title.as_str()));
}
