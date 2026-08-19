//! Branch and directory naming for provisioned worktrees.
//!
//! Branches follow `<type>/<slug>-<id>`; the type is inferred from the task
//! title as a *default* — the user can override the whole name at provision
//! time (`BranchSpec.branch_name`). Explorers get `explorer/<slug>`. Every
//! worktree directory is `<project>@<branch-slug>`, so several worktrees of
//! one repo can sit side by side in a session.

use crate::core::db::models::{Session, SessionKind};

const SLUG_MAX_CHARS: usize = 32;

/// The default branch name for a session's new worktree.
pub fn default_branch(session: &Session) -> String {
    match session.kind {
        SessionKind::Explorer => {
            let slug = slug(&session.title);
            if slug.is_empty() {
                format!("explorer/{}", session.id.trim_start_matches("explorer-"))
            } else {
                format!("explorer/{slug}")
            }
        }
        _ => {
            let id = session.id.to_lowercase();
            let slug = slug(&session.title);
            match slug.is_empty() {
                true => format!("{}/{id}", branch_type(&session.title)),
                false => format!("{}/{slug}-{id}", branch_type(&session.title)),
            }
        }
    }
}

/// `<project>@<branch-slug>` — the one directory shape every worktree gets.
pub fn worktree_dir(project: &str, branch: &str) -> String {
    format!("{project}@{}", branch.replace('/', "-"))
}

/// Conventional-commit type, guessed from whole words of the title. A guess is
/// fine: it only seeds the editable default.
fn branch_type(title: &str) -> &'static str {
    let lowered = title.to_lowercase();
    let words: Vec<&str> = lowered
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let any = |candidates: &[&str]| words.iter().any(|w| candidates.contains(w));

    if any(&["fix", "fixes", "bug", "bugfix", "broken", "crash", "regression"]) {
        "fix"
    } else if any(&["refactor", "rework", "cleanup", "restructure"]) {
        "refactor"
    } else if any(&["docs", "doc", "documentation", "readme"]) {
        "docs"
    } else if any(&["test", "tests", "coverage"]) {
        "test"
    } else if any(&["perf", "performance", "optimize", "optimise", "speedup"]) {
        "perf"
    } else if any(&["upgrade", "bump", "chore", "deps"]) {
        "chore"
    } else {
        "feat"
    }
}

/// Lowercase, alphanumerics joined by single dashes, capped without cutting a
/// word in half.
fn slug(text: &str) -> String {
    let mut out = String::new();
    for word in text
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
    {
        if out.len() + word.len() + 1 > SLUG_MAX_CHARS {
            break;
        }
        if !out.is_empty() {
            out.push('-');
        }
        out.push_str(word);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::models::{SessionKind, SessionState};

    fn session(kind: SessionKind, id: &str, title: &str) -> Session {
        Session {
            id: id.into(),
            kind,
            state: SessionState::Open,
            title: title.into(),
            notion_page_id: (kind == SessionKind::Task).then(|| "page".into()),
            review_project: (kind == SessionKind::Review).then(|| "g/p".into()),
            review_iid: (kind == SessionKind::Review).then_some(1),
            created_at: 0,
        }
    }

    #[test]
    fn task_branches_follow_type_slug_id() {
        let s = session(SessionKind::Task, "TASKS2-42", "Fix the diff parser crash");
        assert_eq!(default_branch(&s), "fix/fix-the-diff-parser-crash-tasks2-42");

        let s = session(SessionKind::Task, "TASKS2-43", "Add MR templates");
        assert_eq!(default_branch(&s), "feat/add-mr-templates-tasks2-43");

        let unnamed = session(SessionKind::Task, "TASKS2-44", "!!!");
        assert_eq!(default_branch(&unnamed), "feat/tasks2-44");
    }

    #[test]
    fn explorer_branches_use_the_session_name() {
        let s = session(SessionKind::Explorer, "explorer-ab12cd34", "Try sqlite vacuum");
        assert_eq!(default_branch(&s), "explorer/try-sqlite-vacuum");

        let unnamed = session(SessionKind::Explorer, "explorer-ab12cd34", "!!!");
        assert_eq!(default_branch(&unnamed), "explorer/ab12cd34");
    }

    #[test]
    fn slugs_are_bounded_and_never_cut_words() {
        let s = session(
            SessionKind::Task,
            "TASKS2-1",
            "Implement the extraordinarily long specification document end to end",
        );
        let branch = default_branch(&s);
        assert!(branch.starts_with("feat/implement-the"), "{branch}");
        assert!(branch.ends_with("-tasks2-1"), "{branch}");
        assert!(branch.len() < 60, "{branch}");
    }

    #[test]
    fn worktree_dirs_carry_the_branch() {
        assert_eq!(worktree_dir("mayo", "fix/tasks2-42-parser"), "mayo@fix-tasks2-42-parser");
        assert_eq!(worktree_dir("mayo", "explorer/x"), "mayo@explorer-x");
    }
}
