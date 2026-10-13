//! A branch named for its session.

use crate::{ExternalId, Session, SessionId, SessionKind, Timestamp, names_session};

fn task(id: &str) -> Session {
    Session {
        id: SessionId::new(id),
        title: "Fix the diff parser crash".into(),
        kind: SessionKind::Task {
            external_id: ExternalId::new("page"),
        },
        created_at: Timestamp::new(0),
    }
}

#[test]
fn only_this_sessions_own_branch_names_it() {
    let s = task("gh-groove-50");
    assert!(names_session(
        "fix/diff-parser-crash-gh-groove-50",
        &s,
        None
    ));
    assert!(names_session("Fix/GH-GROOVE-50", &s, None));
    assert!(names_session("fix/parser-plat-42", &s, Some("PLAT-42")));
    assert!(!names_session("fix/parser-plat-42", &s, Some("")));
    assert!(!names_session("fix/other-gh-groove-49", &s, None));
    assert!(!names_session("fix/other-gh-groove-500", &s, None));
}

#[test]
fn an_explorer_s_branch_is_its_own_name() {
    let mut s = task("explorer-ab12cd34");
    s.kind = crate::SessionKind::Explorer;
    assert_eq!(crate::explorer_branch(&s.id), "explorer/ab12cd34");
    assert!(names_session("explorer/ab12cd34", &s, None));
    assert!(!names_session("explorer/ff00ff00", &s, None));
}
