//! A maintenance task reads the firing alerts, and spawns one child per alert.

use groove_types::{Alert, ClusterConfig, SessionId, Timestamp};

use crate::tests::fixture::until;
use crate::tests::tools::children::{Maintaining, maintaining};
use crate::{Command, alert, dispatch};

fn firing(key: &str, cluster: &str) -> Alert {
    Alert {
        id: format!("id-{key}"),
        title: format!("VeleroBackupFailed {key}"),
        description: "Backup daily-full ended PartiallyFailed.".into(),
        dedup_key: key.into(),
        source_url: Some("https://thanos.example.com/graph".into()),
        priority: Some("Critical".into()),
        cluster: Some(cluster.into()),
        namespace: Some("velero".into()),
        created_at: Timestamp::new(0),
    }
}

/// A marked task whose alerts read `alerts`, and the read landed.
fn alerted(alerts: Vec<Alert>) -> Maintaining {
    let mut at = maintaining(false);
    at.services.alert.hold(alerts);
    let config = at.state.config.config.as_mut().expect("a config");
    config.clusters.push(ClusterConfig::new("prod-eu", &[]));
    let mark = crate::session::Command::SetMaintenance {
        session: at.parent.clone(),
        on: true,
    };
    dispatch(
        Command::Session(mark),
        &mut at.state,
        &at.services,
        &at.spawner,
    );
    until(&at.spawner, &at.services, &mut at.state, |s| {
        s.alert.read_at.is_some()
    });
    at
}

fn investigate(at: &mut Maintaining, key: &str) {
    let asked = alert::Command::Investigate {
        parent: at.parent.clone(),
        dedup_key: key.into(),
    };
    dispatch(
        Command::Alert(asked),
        &mut at.state,
        &at.services,
        &at.spawner,
    );
    until(&at.spawner, &at.services, &mut at.state, |_| {
        at.spawner.idle()
    });
}

fn children(at: &Maintaining) -> Vec<SessionId> {
    let parent = at.state.session.get(&at.parent).expect("the parent");
    parent.children.iter().map(|one| one.id.clone()).collect()
}

#[test]
fn a_task_marked_as_maintenance_reads_the_firing_alerts() {
    let at = alerted(vec![firing("a", "prod-eu"), firing("b", "prod-eu")]);
    assert_eq!(at.state.alert.firing.len(), 2);
    assert!(alert::polls(&at.state));
}

#[test]
fn investigate_spawns_one_child_per_alert_on_the_alert_s_cluster() {
    let mut at = alerted(vec![firing("a", "prod-eu")]);
    investigate(&mut at, "a");
    let spawned = children(&at);
    assert_eq!(spawned.len(), 1);
    let child = at
        .state
        .session
        .get(&spawned[0])
        .expect("the child on the rail");
    assert_eq!(child.session.title, "VeleroBackupFailed a");
    assert_eq!(child.clusters[0].context, "prod-eu");
    assert_eq!(child.clusters[0].namespace.as_deref(), Some("velero"));
    assert_eq!(at.state.session.selected.as_ref(), Some(&spawned[0]));

    investigate(&mut at, "a");
    assert_eq!(children(&at), spawned, "the same child, opened again");
    let store = at.services.session.store();
    let kept = at
        .spawner
        .block_on(store.alert_children(&at.parent))
        .expect("a read");
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].session, spawned[0]);
    let clusters = at
        .spawner
        .block_on(store.clusters_of(&spawned[0]))
        .expect("a read");
    assert_eq!(
        clusters.len(),
        1,
        "the cluster written after the child's row"
    );
}

#[test]
fn an_alert_on_a_cluster_groove_does_not_know_attaches_none() {
    let mut at = alerted(vec![firing("a", "unknown-cluster")]);
    investigate(&mut at, "a");
    let child = children(&at)[0].clone();
    assert!(
        at.state
            .session
            .get(&child)
            .expect("the child")
            .clusters
            .is_empty()
    );
    assert!(at.state.errors.is_empty(), "{:?}", at.state.errors);
}
