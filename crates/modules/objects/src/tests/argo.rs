use groove_types::{KubeKind, TableColumn, Timestamp};

use crate::argo::{columns, row};

fn applications() -> KubeKind {
    KubeKind {
        group: "argoproj.io".into(),
        version: "v1alpha1".into(),
        kind: "Application".into(),
        plural: "applications".into(),
        namespaced: true,
        watchable: true,
    }
}

fn server() -> Vec<TableColumn> {
    let column = |name: &str, priority| TableColumn {
        name: name.into(),
        priority,
        date: name == "Age",
    };
    vec![
        column("Name", 0),
        column("Sync Status", 0),
        column("Health Status", 0),
        column("Revision", 10),
        column("Project", 10),
        column("Age", 0),
    ]
}

#[test]
fn an_application_lists_its_project_sync_health_auto_sync_and_short_revision() {
    let kind = applications();
    let names: Vec<String> = columns(&kind, &server())
        .into_iter()
        .map(|one| one.name)
        .collect();
    assert_eq!(
        names,
        [
            "Name",
            "Project",
            "Sync Status",
            "Health Status",
            "Auto-sync",
            "Revision"
        ]
    );
    let raw = groove_kube::Row {
        uid: "u1".into(),
        name: "pythie-cayzn-acc".into(),
        namespace: Some("argocd".into()),
        version: "9".into(),
        created: None,
        cells: [
            "pythie-cayzn-acc",
            "OutOfSync",
            "Degraded",
            "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678",
            "data-science",
            "41d",
        ]
        .map(String::from)
        .to_vec(),
        object: Some(serde_json::json!({
            "spec": { "syncPolicy": { "automated": { "prune": true, "selfHeal": true } } },
            "status": { "sync": { "revisions": ["3.8.0", "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678"] } }
        })),
    };
    let listed = row(&kind, raw, &server(), Timestamp::new(0));
    assert_eq!(
        listed.cells,
        [
            "pythie-cayzn-acc",
            "data-science",
            "OutOfSync",
            "Degraded",
            "auto · prune · heal",
            "3.8.0 · a1b2c3d"
        ]
    );
    assert!(listed.aging.is_empty(), "no age column to count up");
}

#[test]
fn an_application_without_a_sync_policy_is_manual_and_a_tag_reads_whole() {
    assert_eq!(
        crate::argo::automated(Some(&serde_json::json!({}))),
        "manual"
    );
    let partial = serde_json::json!({ "syncPolicy": { "automated": { "prune": false } } });
    assert_eq!(crate::argo::automated(Some(&partial)), "auto");
    assert_eq!(crate::argo::short("v0.24.1"), "v0.24.1");
}

#[test]
fn a_failed_application_reads_its_sources_policy_operation_and_what_broke() {
    let app = serde_json::json!({
        "apiVersion": "argoproj.io/v1alpha1", "kind": "Application",
        "metadata": { "name": "pythie-cayzn-acc", "namespace": "argocd", "uid": "u1" },
        "spec": {
            "project": "data-science",
            "sources": [
                { "repoURL": "https://gitlab.wiremind.io/charts.git", "chart": "pythie", "targetRevision": "3.8.0",
                  "helm": { "valueFiles": ["$values/envs/acc/values.yaml"] } },
                { "repoURL": "https://gitlab.wiremind.io/pythie-cayzn-deploy.git", "targetRevision": "main", "ref": "values" }
            ],
            "destination": { "server": "https://10.0.0.1:6443", "namespace": "pythie-cayzn-acc" },
            "syncPolicy": { "automated": { "prune": true, "selfHeal": true } }
        },
        "status": {
            "sync": { "status": "OutOfSync", "revision": "e9f8a7b" },
            "health": { "status": "Degraded" },
            "conditions": [{ "type": "SyncError", "message": "one or more synchronization tasks completed unsuccessfully",
                             "lastTransitionTime": "2026-10-10T09:00:00Z" }],
            "operationState": {
                "phase": "Failed", "message": "one or more objects failed to apply", "retryCount": 2,
                "startedAt": "2026-10-10T08:56:00Z", "finishedAt": "2026-10-10T08:56:38Z",
                "operation": { "initiatedBy": { "automated": true } },
                "syncResult": { "revision": "a1b2c3d", "resources": [
                    { "kind": "Job", "name": "pythie-migrate", "namespace": "pythie-cayzn-acc", "status": "Synced",
                      "hookType": "PreSync", "hookPhase": "Failed", "message": "Job has reached the specified backoff limit" },
                    { "kind": "ConfigMap", "name": "pythie-config", "namespace": "pythie-cayzn-acc", "status": "Synced", "message": "configured" }
                ] }
            }
        }
    });
    let part = crate::describe("Application", &app, false)
        .app
        .expect("an application's part");
    assert_eq!(
        (
            part.project.as_str(),
            part.sync.as_str(),
            part.health.as_str()
        ),
        ("data-science", "OutOfSync", "Degraded")
    );
    assert_eq!(part.sources.len(), 2);
    assert_eq!(part.sources[0].values, ["$values/envs/acc/values.yaml"]);
    assert_eq!(
        part.destination.server.as_deref(),
        Some("https://10.0.0.1:6443")
    );
    assert_eq!(part.automated, Some((true, true)));
    let operation = part.operation.expect("its last sync");
    assert!(operation.failed());
    assert_eq!((operation.by.as_str(), operation.retries), ("auto-sync", 2));
    assert_eq!(operation.revision.as_deref(), Some("a1b2c3d"));
    let broke: Vec<&str> = operation
        .failed
        .iter()
        .map(|one| one.name.as_str())
        .collect();
    assert_eq!(
        broke,
        ["pythie-migrate"],
        "the failed hook, not the configured map"
    );
    assert!(operation.failed[0].hook);
    assert_eq!(part.conditions[0].kind, "SyncError");
    let other = serde_json::json!({ "apiVersion": "app.k8s.io/v1beta1", "kind": "Application" });
    assert!(
        crate::describe("Application", &other, false).app.is_none(),
        "another group's kind"
    );
}
