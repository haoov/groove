use crate::Stored as _;

use super::store::{explorer, promoted, task};
use crate::Store;

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
