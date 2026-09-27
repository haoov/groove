//! Branch names: the default for a session, and what git would refuse.

use groove_types::{Session, SessionKind};

use crate::{Error, Result};

const SLUG_MAX_CHARS: usize = 32;
const FORBIDDEN_CHARS: [char; 8] = [' ', '~', '^', ':', '?', '*', '[', '\\'];

/// `<type>/<slug>-<id>` for a task, `explorer/<slug>` for an explorer; `tag` replaces the id.
pub fn default_branch(session: &Session, tag: Option<&str>) -> String {
    let slug = slug(&session.title);
    match session.kind {
        SessionKind::Explorer if slug.is_empty() => {
            format!(
                "explorer/{}",
                session.id.as_str().trim_start_matches("explorer-")
            )
        }
        SessionKind::Explorer => format!("explorer/{slug}"),
        _ => {
            let id = tag
                .filter(|t| !t.is_empty())
                .unwrap_or(session.id.as_str())
                .to_lowercase();
            let kind = branch_type(&session.title);
            if slug.is_empty() {
                format!("{kind}/{id}")
            } else {
                format!("{kind}/{slug}-{id}")
            }
        }
    }
}

/// The conventional-commit type the title's words suggest.
fn branch_type(title: &str) -> &'static str {
    let lowered = title.to_lowercase();
    let words: Vec<&str> = lowered
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let any = |candidates: &[&str]| words.iter().any(|w| candidates.contains(w));
    if any(&[
        "fix",
        "fixes",
        "bug",
        "bugfix",
        "broken",
        "crash",
        "regression",
    ]) {
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

/// Lowercase alphanumerics joined by dashes, capped without cutting a word.
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

/// What `git check-ref-format --branch` would refuse.
pub fn validate_branch_name(branch: &str) -> Result<()> {
    let bad = |reason: &str| Error::InvalidBranch {
        branch: branch.to_string(),
        reason: reason.to_string(),
    };
    if branch.is_empty() {
        return Err(bad("empty"));
    }
    if branch.starts_with('/') || branch.ends_with('/') {
        return Err(bad("leading or trailing '/'"));
    }
    if branch.contains("//") {
        return Err(bad("empty path component"));
    }
    if branch.starts_with('-') {
        return Err(bad("leading '-'"));
    }
    if branch.contains("..") || branch.contains("@{") {
        return Err(bad("'..' or '@{'"));
    }
    if let Some(c) = branch
        .chars()
        .find(|c| c.is_ascii_control() || FORBIDDEN_CHARS.contains(c))
    {
        return Err(bad(&format!("forbidden character {c:?}")));
    }
    for component in branch.split('/') {
        if component.starts_with('.') {
            return Err(bad("component starting with '.'"));
        }
        if component.ends_with(".lock") {
            return Err(bad("component ending in '.lock'"));
        }
        if component.ends_with('.') {
            return Err(bad("component ending in '.'"));
        }
    }
    Ok(())
}
