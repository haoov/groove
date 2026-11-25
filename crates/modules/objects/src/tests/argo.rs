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
        short_names: Vec::new(),
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
        name: "billing-eu-acc".into(),
        namespace: Some("argocd".into()),
        version: "9".into(),
        created: None,
        cells: [
            "billing-eu-acc",
            "OutOfSync",
            "Degraded",
            "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678",
            "analytics",
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
            "billing-eu-acc",
            "analytics",
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
    assert_eq!(groove_types::short_sha("v0.24.1"), "v0.24.1");
}

#[test]
fn a_failed_application_reads_its_sources_policy_operation_and_what_broke() {
    let app = serde_json::json!({
        "apiVersion": "argoproj.io/v1alpha1", "kind": "Application",
        "metadata": { "name": "billing-eu-acc", "namespace": "argocd", "uid": "u1" },
        "spec": {
            "project": "analytics",
            "sources": [
                { "repoURL": "https://gitlab.example.com/charts.git", "chart": "billing", "targetRevision": "3.8.0",
                  "helm": { "valueFiles": ["$values/envs/acc/values.yaml"] } },
                { "repoURL": "https://gitlab.example.com/billing-eu-deploy.git", "targetRevision": "main", "ref": "values" }
            ],
            "destination": { "server": "https://10.0.0.1:6443", "namespace": "billing-eu-acc" },
            "syncPolicy": { "automated": { "prune": true, "selfHeal": true } }
        },
        "status": {
            "sync": { "status": "OutOfSync", "revision": "e9f8a7b" },
            "health": { "status": "Degraded" },
            "resources": [
                { "group": "apps", "kind": "Deployment", "namespace": "billing-eu-acc", "name": "billing-api",
                  "status": "OutOfSync", "syncWave": 1, "health": { "status": "Progressing", "message": "1 of 3 updated" } },
                { "kind": "ConfigMap", "namespace": "billing-eu-acc", "name": "billing-old", "status": "OutOfSync",
                  "requiresPruning": true },
                { "group": "batch", "kind": "Job", "namespace": "billing-eu-acc", "name": "billing-migrate",
                  "status": "OutOfSync", "hook": true, "requiresPruning": true }
            ],
            "conditions": [{ "type": "SyncError", "message": "one or more synchronization tasks completed unsuccessfully",
                             "lastTransitionTime": "2026-10-10T09:00:00Z" }],
            "operationState": {
                "phase": "Failed", "message": "one or more objects failed to apply", "retryCount": 2,
                "startedAt": "2026-10-10T08:56:00Z", "finishedAt": "2026-10-10T08:56:38Z",
                "operation": { "initiatedBy": { "automated": true } },
                "syncResult": { "revision": "a1b2c3d", "resources": [
                    { "kind": "Job", "name": "billing-migrate", "namespace": "billing-eu-acc", "status": "Synced",
                      "hookType": "PreSync", "hookPhase": "Failed", "syncPhase": "PreSync", "message": "Job has reached the specified backoff limit" },
                    { "kind": "ConfigMap", "name": "billing-config", "namespace": "billing-eu-acc", "status": "Synced", "message": "configured" }
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
        ("analytics", "OutOfSync", "Degraded")
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
        ["billing-migrate"],
        "the failed hook, not the configured map"
    );
    assert!(operation.failed[0].hook);
    assert_eq!(
        operation.applied.len(),
        2,
        "every result, the failed hook among them"
    );
    assert_eq!(operation.applied[0].phase, "PreSync");
    let api = &part.resources[0];
    assert_eq!((api.wave, api.out_of_sync()), (1, true));
    let old = &part.resources[1];
    assert!(old.prune && old.out_of_sync());
    assert!(
        part.resources[2].hook && !part.resources[2].out_of_sync() && !part.resources[2].prune,
        "a hook's status and pruning count for nothing"
    );
    assert_eq!(part.conditions[0].kind, "SyncError");
    let other = serde_json::json!({ "apiVersion": "app.k8s.io/v1beta1", "kind": "Application" });
    assert!(
        crate::describe("Application", &other, false).app.is_none(),
        "another group's kind"
    );
}

#[test]
fn a_source_keeps_its_last_synced_commit_apart_from_the_one_compared() {
    let sha = "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678";
    let app = serde_json::json!({
        "apiVersion": "argoproj.io/v1alpha1", "kind": "Application",
        "metadata": { "name": "billing-eu-acc", "namespace": "argocd", "uid": "u1" },
        "spec": { "source": { "repoURL": "https://gitlab.example.com/billing-eu-deploy.git", "targetRevision": "main" } },
        "status": {
            "sync": { "status": "OutOfSync", "revision": "f00d" },
            "history": [{ "revision": "0ld" }, { "revision": sha }]
        }
    });
    let part = crate::describe("Application", &app, false)
        .app
        .expect("an application's part");
    assert_eq!(
        part.sources[0].synced.as_deref(),
        Some(sha),
        "the last sync applied"
    );
    assert_eq!(
        part.sources[0].compared.as_deref(),
        Some("f00d"),
        "what it compares against"
    );
}

#[test]
fn an_empty_sources_list_leaves_the_one_source_read() {
    let app = serde_json::json!({
        "apiVersion": "argoproj.io/v1alpha1", "kind": "Application",
        "metadata": { "name": "billing-eu-acc", "namespace": "argocd", "uid": "u1" },
        "spec": { "sources": [], "source": { "repoURL": "https://gitlab.example.com/deploy.git", "targetRevision": "main" } },
        "status": { "sync": { "status": "Synced" } }
    });
    let part = crate::describe("Application", &app, false)
        .app
        .expect("an application's part");
    assert_eq!(part.sources.len(), 1);
}

#[test]
fn a_401_through_the_hub_is_a_refused_login_and_a_502_is_not() {
    let api = |status| groove_kube::Error::Api {
        context: "hub".into(),
        status,
        message: "token is expired".into(),
    };
    let streamed = groove_kube::Error::Refused {
        context: "hub".into(),
        detail: "token is expired".into(),
    };
    assert!(crate::argo_tree::login_refused(&api(401)));
    assert!(crate::argo_tree::login_refused(&streamed));
    assert!(!crate::argo_tree::login_refused(&api(502)));
}

#[test]
fn the_sync_policy_reads_its_options_and_its_retry() {
    let app = serde_json::json!({
        "apiVersion": "argoproj.io/v1alpha1", "kind": "Application",
        "metadata": { "name": "billing-eu-acc", "namespace": "argocd", "uid": "u1" },
        "spec": { "syncPolicy": {
            "syncOptions": ["ServerSideApply=true", "RespectIgnoreDifferences=true"],
            "retry": { "limit": 5, "backoff": { "duration": "5s", "factor": 2, "maxDuration": "3m" } }
        } }
    });
    let part = crate::describe("Application", &app, false)
        .app
        .expect("an application's part");
    assert_eq!(part.automated, None);
    assert_eq!(
        part.options,
        ["ServerSideApply=true", "RespectIgnoreDifferences=true"]
    );
    let retry = part.retry.expect("its retry");
    assert_eq!((retry.limit, retry.factor), (5, Some(2)));
    assert_eq!(
        (retry.backoff.as_deref(), retry.max.as_deref()),
        (Some("5s"), Some("3m"))
    );
}
