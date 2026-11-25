use crate::{Resource, refresh_path, sync_body, sync_path, taken};

#[test]
fn a_sync_never_names_sync_options_so_the_application_s_own_apply() {
    let body = sync_body("argocd", false, &[]);
    let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert!(json.get("syncOptions").is_none(), "{json}");
    assert_eq!(json["prune"], false);
    assert!(json.get("resources").is_none(), "the whole Application");
    assert_eq!(sync_path("shop"), "/api/v1/applications/shop/sync");
}

#[test]
fn a_sync_of_some_resources_names_each_with_its_group() {
    let web = Resource {
        group: "apps".into(),
        kind: "Deployment".into(),
        namespace: Some("shop".into()),
        name: "web".into(),
    };
    let body = sync_body("argocd", true, &[web]);
    let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(json["prune"], true);
    assert_eq!(json["resources"][0]["group"], "apps");
    assert_eq!(json["resources"][0]["namespace"], "shop");
}

#[test]
fn an_application_answered_is_taken_and_a_refusal_says_why() {
    assert_eq!(taken(r#"{"metadata":{"name":"shop"}}"#), Ok(()));
    let refused = r#"{"code":7,"message":"permission denied: applications, sync, shop/web"}"#;
    assert_eq!(
        taken(refused),
        Err("permission denied: applications, sync, shop/web".into())
    );
    let hard = refresh_path("argocd", "shop", true);
    assert!(hard.ends_with("&refresh=hard"), "{hard}");
}
