use crate::{revision, revision_path};

#[test]
fn a_commit_reads_its_author_without_address_and_its_first_line() {
    let body = r#"{"author":"Jane Doe <jane@example.com>","date":"2026-11-20T10:00:00Z",
        "message":"chore(billing): bump api\n\nlong body","tags":["v1"]}"#;
    let read = revision(body).expect("a revision");
    assert_eq!(read.author, "Jane Doe");
    assert_eq!(read.message, "chore(billing): bump api");
    assert!(read.date.is_some());
}

#[test]
fn a_refusal_reads_as_what_argo_cd_said() {
    let body = r#"{"error":"permission denied","code":7,"message":"permission denied"}"#;
    assert_eq!(revision(body), Err("permission denied".to_string()));
}

#[test]
fn a_source_of_several_is_named_by_its_index() {
    let one = revision_path("argocd", "shop", "a1b2", None);
    assert_eq!(
        one,
        "/api/v1/applications/shop/revisions/a1b2/metadata?appNamespace=argocd"
    );
    let second = revision_path("argocd", "shop", "a1b2", Some(1));
    assert!(second.ends_with("&sourceIndex=1"), "{second}");
}
