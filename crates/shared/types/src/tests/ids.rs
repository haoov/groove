//! Ids, and the names that parse both ways.

use crate::{ErrorKind, Forge, ProviderId, SessionId, SessionKind, WorktreeId};

#[test]
fn ids_are_transparent_strings_with_a_named_debug() {
    let id = SessionId::new("gh-groove-50");
    assert_eq!(id.to_string(), "gh-groove-50");
    assert_eq!(format!("{id:?}"), "SessionId(gh-groove-50)");
    assert_eq!(serde_json::to_string(&id).unwrap(), "\"gh-groove-50\"");
    let back: WorktreeId = serde_json::from_str("\"wt-1\"").unwrap();
    assert_eq!(back.as_str(), "wt-1");
}

#[test]
fn names_parse_both_ways() {
    assert_eq!(ProviderId::parse("github").unwrap(), ProviderId::Github);
    assert_eq!(
        ProviderId::parse("jira").unwrap_err().kind,
        ErrorKind::Invalid
    );
    assert_eq!(Forge::parse("gitlab").unwrap().as_str(), "gitlab");
    assert_eq!(SessionKind::Explorer.name(), "explorer");
    let json = serde_json::to_value(SessionKind::Review {
        project: "g/p".into(),
        iid: 7,
    })
    .unwrap();
    assert_eq!(json["kind"], "review");
    assert_eq!(json["iid"], 7);
}
