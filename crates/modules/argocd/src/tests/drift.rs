use crate::{drift, drift_path};

fn item(kind: &str, name: &str, live: &str, desired: &str, modified: bool, hook: bool) -> String {
    let quoted = |state: &str| serde_json::to_string(state).unwrap_or_default();
    format!(
        r#"{{"group":"apps","kind":"{kind}","namespace":"shop","name":"{name}",
            "normalizedLiveState":{},"predictedLiveState":{},"modified":{modified},"hook":{hook}}}"#,
        quoted(live),
        quoted(desired),
    )
}

#[test]
fn only_what_differs_reads_hooks_and_resources_in_sync_left_out() {
    let web = r#"{"spec":{"replicas":2}}"#;
    let wanted = r#"{"spec":{"replicas":3}}"#;
    let body = format!(
        r#"{{"items":[{},{},{},{},{}]}}"#,
        item("Deployment", "web", web, wanted, true, false),
        item("Deployment", "api", web, web, false, false),
        item("ConfigMap", "old", web, "null", true, false),
        item("Service", "new", "null", wanted, true, false),
        item("Job", "migrate", web, wanted, true, true),
    );
    let read = drift(&body).expect("a list");
    let names: Vec<&str> = read.iter().map(|one| one.name.as_str()).collect();
    assert_eq!(names, ["web", "old", "new"]);
    assert_eq!(
        read[0]
            .desired
            .as_ref()
            .map(|one| one["spec"]["replicas"].clone()),
        Some(3.into())
    );
    assert!(
        read[1].desired.is_none(),
        "a resource to prune has no desired side"
    );
    assert!(
        read[2].live.is_none(),
        "a resource missing live has no live side"
    );
}

#[test]
fn a_refusal_reads_as_what_argo_cd_said() {
    let refused =
        r#"{"error":"no session information","code":16,"message":"no session information"}"#;
    assert_eq!(drift(refused), Err("no session information".to_string()));
}

#[test]
fn the_managed_resources_are_asked_of_one_application() {
    assert_eq!(
        drift_path("argocd", "shop"),
        "/api/v1/applications/shop/managed-resources?appNamespace=argocd"
    );
}
