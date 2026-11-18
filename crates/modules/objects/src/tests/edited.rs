use groove_types::ErrorKind;
use serde_json::json;

use crate::edited;
use crate::edited::parsed;
use crate::yaml::yaml;

fn api() -> serde_json::Value {
    json!({ "metadata": { "name": "api", "resourceVersion": "7",
        "labels": { "app": "api", "tier": "web" } },
        "spec": { "replicas": 1, "paused": false, "note": "3", "answer": "no",
            "script": "set -e\nrun\n", "ports": [{ "port": 80 }] },
        "status": { "ready": 1 } })
}

#[test]
fn the_yaml_groove_writes_reads_back_as_the_same_object() {
    assert_eq!(parsed(&yaml(&api())).expect("read back"), api());
}

#[test]
fn an_edit_patches_what_changed_and_nulls_what_went() {
    let before = yaml(&api());
    let after = before
        .replace("replicas: 1", "replicas: 3")
        .replace("    tier: web\n", "");
    let patch = edited(&before, &after).expect("a patch");
    assert_eq!(
        patch,
        json!({ "metadata": { "labels": { "tier": null } }, "spec": { "replicas": 3 } })
    );
}

#[test]
fn an_edit_of_what_the_server_writes_is_refused_and_says_which() {
    let before = yaml(&api());
    let after = before.replace("ready: 1", "ready: 2");
    let refused = edited(&before, &after).expect_err("status is the server's");
    assert_eq!(refused.kind, ErrorKind::Invalid);
    assert!(refused.message.contains("status"), "{}", refused.message);
    let after = before.replace("resourceVersion: \"7\"", "resourceVersion: \"8\"");
    let refused = edited(&before, &after).expect_err("the version is the server's");
    assert!(refused.message.contains("metadata.resourceVersion"));
}

#[test]
fn an_edit_that_reads_badly_or_changes_nothing_is_refused() {
    let before = yaml(&api());
    for (after, said) in [
        (before.clone(), "changes nothing"),
        (format!("{before}  - broken: [\n"), "does not read"),
        ("---\na: 1\n---\nb: 2\n".to_string(), "one document"),
    ] {
        let refused = edited(&before, &after).expect_err(said);
        assert!(refused.message.contains(said), "{}", refused.message);
    }
}
