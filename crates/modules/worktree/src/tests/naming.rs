use groove_types::{ExternalId, Session, SessionId, SessionKind, Timestamp};

use crate::layout::worktree_leaf;
use crate::naming::{default_branch, validate_branch_name};

fn session(kind: SessionKind, id: &str, title: &str) -> Session {
    Session {
        id: SessionId::new(id),
        title: title.into(),
        kind,
        created_at: Timestamp::new(0),
    }
}

fn task(id: &str, title: &str) -> Session {
    session(
        SessionKind::Task {
            external_id: ExternalId::new("page"),
        },
        id,
        title,
    )
}

#[test]
fn task_branches_follow_type_slug_id() {
    assert_eq!(
        default_branch(&task("TASKS2-42", "Fix the diff parser crash"), None),
        "fix/fix-the-diff-parser-crash-tasks2-42"
    );
    assert_eq!(
        default_branch(&task("TASKS2-43", "Add MR templates"), None),
        "feat/add-mr-templates-tasks2-43"
    );
    assert_eq!(
        default_branch(&task("TASKS2-44", "!!!"), None),
        "feat/tasks2-44"
    );
    assert_eq!(
        default_branch(
            &task("gh-groove-42", "Fix the diff parser crash"),
            Some("42")
        ),
        "fix/fix-the-diff-parser-crash-42"
    );
    assert_eq!(
        default_branch(&task("T-1", "Bump deps"), None),
        "chore/bump-deps-t-1"
    );
}

#[test]
fn explorer_branches_use_the_title_or_the_id() {
    let s = session(
        SessionKind::Explorer,
        "explorer-ab12cd34",
        "Try sqlite vacuum",
    );
    assert_eq!(default_branch(&s, None), "explorer/try-sqlite-vacuum");
    let unnamed = session(SessionKind::Explorer, "explorer-ab12cd34", "!!!");
    assert_eq!(default_branch(&unnamed, None), "explorer/ab12cd34");
}

#[test]
fn slugs_are_bounded_and_never_cut_words() {
    let branch = default_branch(
        &task(
            "TASKS2-1",
            "Implement the extraordinarily long specification document end to end",
        ),
        None,
    );
    assert!(branch.starts_with("feat/implement-the"), "{branch}");
    assert!(branch.ends_with("-tasks2-1"), "{branch}");
    assert!(branch.len() < 60, "{branch}");
}

#[test]
fn a_slash_and_a_dash_are_different_worktrees() {
    assert_eq!(
        worktree_leaf("mayo", "fix/tasks2-42-parser"),
        std::path::PathBuf::from("mayo/fix/tasks2-42-parser")
    );
    assert_ne!(
        worktree_leaf("mayo", "fix/parser"),
        worktree_leaf("mayo", "fix-parser")
    );
}

#[test]
fn branch_names_git_would_refuse_are_refused() {
    for ok in ["main", "fix/parser", "feat/a.b-c_d", "release/1.0"] {
        validate_branch_name(ok).unwrap_or_else(|e| panic!("{ok}: {e}"));
    }
    for bad in [
        "",
        "/lead",
        "trail/",
        "a//b",
        "-dash",
        "a..b",
        "with space",
        "a:b",
        ".hidden/x",
        "x/y.lock",
        "end.",
        "a@{b",
        "tab\there",
    ] {
        assert!(
            validate_branch_name(bad).is_err(),
            "{bad:?} should be refused"
        );
    }
}
