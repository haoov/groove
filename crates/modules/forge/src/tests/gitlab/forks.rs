//! A fork's MR from a branch of the same name is not this project's.

use super::*;

#[tokio::test]
async fn a_fork_s_mr_on_the_same_branch_is_not_the_branch_s_own() {
    let mut fork = mr("opened");
    fork["iid"] = "9".into();
    fork["sourceProject"] = serde_json::json!({ "fullPath": "someone/overwhelm" });
    let mut own = mr("opened");
    own["sourceProject"] = serde_json::json!({ "fullPath": "wiremind/devops/overwhelm" });
    let (_server, both) = gitlab(by_branch(vec![fork.clone(), own])).await;
    let read = both.find_mr(&repo(), "main").await.unwrap();
    assert_eq!(read.expect("its own mr").number, "7");
    let (_server, alone) = gitlab(by_branch(vec![fork])).await;
    assert!(alone.find_mr(&repo(), "main").await.unwrap().is_none());
}
