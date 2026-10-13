//! A fork's PR from a branch of the same name is not this repo's.

use super::*;

#[tokio::test]
async fn a_fork_s_pr_on_the_same_branch_is_not_the_branch_s_own() {
    let mut fork = pr();
    fork["number"] = 9.into();
    fork["isCrossRepository"] = true.into();
    let mut own = pr();
    own["isCrossRepository"] = false.into();
    let (_server, both) = github(by_branch(vec![fork.clone(), own])).await;
    let read = both.find_mr(&repo(), "main").await.unwrap();
    assert_eq!(read.expect("its own pr").number, "7");
    let (_server, alone) = github(by_branch(vec![fork])).await;
    assert!(alone.find_mr(&repo(), "main").await.unwrap().is_none());
}
