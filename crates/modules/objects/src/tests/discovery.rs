use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::kinds;
use crate::tests::watch::kubeconfig;

#[tokio::test]
async fn the_kinds_are_asked_once_then_read_from_the_disk_until_asked_again() {
    let server = MockServer::start().await;
    let plain = |body: serde_json::Value| ResponseTemplate::new(200).set_body_json(body);
    let versions = serde_json::json!({ "kind": "APIVersions", "versions": ["v1"], "serverAddressByClientCIDRs": [] });
    let resources = serde_json::json!({ "kind": "APIResourceList", "groupVersion": "v1", "resources": [
        { "name": "pods", "singularName": "pod", "namespaced": true, "kind": "Pod", "verbs": ["list", "watch"] }
    ] });
    Mock::given(path("/api"))
        .respond_with(plain(versions))
        .mount(&server)
        .await;
    Mock::given(path("/api/v1"))
        .respond_with(plain(resources))
        .expect(2)
        .mount(&server)
        .await;
    let groups = serde_json::json!({ "kind": "APIGroupList", "groups": [] });
    Mock::given(path("/apis"))
        .respond_with(plain(groups))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().expect("a temp dir");
    let paths = [kubeconfig(dir.path(), &server.uri())];
    let cache = dir.path().join("cache");
    let first = kinds(&paths, "kind", &cache, false).await.expect("asked");
    let second = kinds(&paths, "kind", &cache, false).await.expect("cached");
    assert_eq!(first, second);
    assert_eq!(first[0].plural, "pods");
    assert!(cache.join("kind.json").exists());
    kinds(&paths, "kind", &cache, true)
        .await
        .expect("asked again");
}
