use crate::{health, streamed, tree_path};

const TREE: &str = r#"{"nodes":[
  {"group":"apps","version":"v1","kind":"Deployment","namespace":"shop","name":"web",
   "health":{"status":"Degraded","message":"progress deadline exceeded"}},
  {"version":"v1","kind":"ConfigMap","namespace":"shop","name":"web-config"}
]}"#;

#[test]
fn each_node_reads_with_its_health_and_message() {
    let read = health(TREE).expect("a tree");
    assert_eq!(read.len(), 2);
    assert_eq!(read[0].status, "Degraded");
    assert_eq!(read[0].reason, "progress deadline exceeded");
    assert_eq!(read[1].group, "");
    assert_eq!(
        read[1].status, "",
        "a kind Argo CD keeps no health for has none"
    );
}

#[test]
fn a_refusal_reads_as_what_argo_cd_said() {
    let refused =
        r#"{"error":"no session information","code":16,"message":"no session information"}"#;
    assert_eq!(health(refused), Err("no session information".to_string()));
}

#[test]
fn the_stream_gives_a_tree_a_line_and_its_errors() {
    let line = format!(r#"{{"result":{TREE}}}"#).replace('\n', "");
    let read = streamed(&line).expect("a line").expect("a tree");
    assert_eq!(read[0].name, "web");
    let error = r#"{"error":{"grpc_code":16,"http_code":401,"message":"no session information"}}"#;
    assert_eq!(
        streamed(error),
        Some(Err("no session information".to_string()))
    );
    assert_eq!(streamed(""), None);
}

#[test]
fn the_tree_is_asked_of_one_application_once_or_streamed() {
    assert_eq!(
        tree_path("argocd", "shop", false),
        "/api/v1/applications/shop/resource-tree?appNamespace=argocd"
    );
    assert_eq!(
        tree_path("argocd", "shop", true),
        "/api/v1/stream/applications/shop/resource-tree?appNamespace=argocd"
    );
}
