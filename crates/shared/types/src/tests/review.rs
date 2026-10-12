//! An MR asked for review: its session, its clone URL, and its repo.

use crate::{Forge, Timestamp};

fn asked(project: &str, iid: u64, web_url: &str) -> crate::ReviewMr {
    crate::ReviewMr {
        forge: Forge::Github,
        project: project.into(),
        iid,
        title: "fix: one".into(),
        author: "someone".into(),
        source_branch: "fix/one".into(),
        target_branch: "main".into(),
        draft: false,
        web_url: web_url.into(),
        updated_at: Timestamp::new(0),
        local_path: None,
        approved: false,
        review: None,
    }
}

#[test]
fn an_mr_names_the_session_that_reviews_it_the_same_way_every_time() {
    let one = asked("acme/groove", 7, "https://github.com/acme/groove/pull/7");
    assert_eq!(one.session_id(), "review-acme-groove-7");
    assert_eq!(one.session_id(), asked("ACME/Groove", 7, "").session_id());
}

#[test]
fn an_mrs_own_page_says_where_its_repo_is_cloned_from() {
    let github = asked("acme/groove", 7, "https://github.com/acme/groove/pull/7");
    assert_eq!(
        github.clone_url().as_deref(),
        Some("https://github.com/acme/groove.git")
    );
    let gitlab = asked(
        "devops/charts",
        3,
        "https://gitlab.example.com/devops/charts/-/merge_requests/3",
    );
    assert_eq!(
        gitlab.clone_url().as_deref(),
        Some("https://gitlab.example.com/devops/charts.git")
    );
    assert_eq!(asked("a/b", 1, "").clone_url(), None);
}

#[test]
fn a_review_s_repo_is_the_clone_of_its_own_host_and_project() {
    let entry = |slug: &str| crate::PoolEntry {
        slug: slug.into(),
        path: std::path::PathBuf::from("/pool/p"),
    };
    let mr = crate::ReviewMr {
        forge: crate::Forge::Github,
        project: "acme/groove".into(),
        iid: 7,
        title: String::new(),
        author: String::new(),
        source_branch: String::new(),
        target_branch: String::new(),
        draft: false,
        web_url: "https://github.com/acme/groove/pull/7".into(),
        updated_at: crate::Timestamp::new(0),
        local_path: None,
        approved: false,
        review: None,
    };
    assert!(mr.held_by(&entry("github.com/acme/groove")));
    assert!(
        !mr.held_by(&entry("gitlab.com/acme/groove")),
        "another host"
    );
    assert!(
        !mr.held_by(&entry("github.com/other/acme/groove")),
        "another path"
    );
}
