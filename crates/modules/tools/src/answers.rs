//! What a tool answers, in the names the agent reads.

use groove_types::{
    Annotation, CiStatus, CommitEntry, FileDiff, Mr, MrThread, PoolEntry, Repo, Session, Skill,
    Task, Worktree, WorktreeId, WorktreeStatus,
};
use serde::Serialize;
use serde_json::{Value, json};

/// A worktree as every answer names it, with what this answer adds.
pub fn worktree(one: &Worktree, adds: Value) -> Value {
    let mut out = json!({
        "worktree_id": one.id,
        "repo": one.repo,
        "branch": one.branch,
        "target_branch": one.base_ref,
        "path": one.path,
    });
    if let (Some(out), Value::Object(adds)) = (out.as_object_mut(), adds) {
        out.extend(adds);
    }
    out
}

/// Several worktrees, each as `worktree` names it.
pub fn worktrees(each: Vec<Value>) -> Value {
    json!({ "worktrees": each })
}

pub fn changed(one: &Worktree, against: &str, files: &[FileDiff]) -> Value {
    worktree(one, json!({ "against": against, "files": files }))
}

pub fn commits(one: &Worktree, log: &[CommitEntry]) -> Value {
    worktree(one, json!({ "commits": log }))
}

pub fn status(one: &Worktree, told: &WorktreeStatus) -> Value {
    let adds = json!({
        "modified": told.modified,
        "staged": told.staged,
        "ahead": told.ahead,
        "behind": told.behind,
    });
    worktree(one, adds)
}

/// The session the call comes from, with its repos and its worktrees.
pub fn session(one: &Session, auto_approve: bool, repos: &[Repo], worktrees: &[Worktree]) -> Value {
    json!({
        "session": one,
        "auto_approve": auto_approve,
        "repos": repos.iter().map(repo).collect::<Vec<Value>>(),
        "worktrees": worktrees.iter().map(|w| worktree(w, json!({}))).collect::<Vec<Value>>(),
    })
}

pub fn repo(one: &Repo) -> Value {
    json!({
        "id": one.id,
        "repo": one.slug(),
        "project": one.project,
        "local_path": one.local_path,
    })
}

/// A clone of the pool, and whether the session already holds it.
pub fn pooled(one: &PoolEntry, attached: bool) -> Value {
    json!({ "repo": one.slug, "local_path": one.path, "attached": attached })
}

pub fn task(one: &Task) -> Value {
    json!({
        "task_id": one.short_id,
        "external_id": one.external_id,
        "title": one.title,
        "status": one.status,
        "priority": one.priority,
        "url": one.url,
    })
}

/// A task's page as its source holds it.
pub fn body(one: &Task, markdown: &str) -> Value {
    json!({
        "task_id": one.short_id,
        "title": one.title,
        "url": one.url,
        "body_markdown": markdown,
    })
}

/// The headings a new task mirrors, and where it is filed.
pub fn template(markdown: &str, file_at: impl Serialize) -> Value {
    json!({ "template_markdown": markdown, "file_at": file_at })
}

pub fn skill(one: &Skill) -> Value {
    json!({
        "id": one.id,
        "name": one.name,
        "description": one.description,
        "yours": one.editable,
    })
}

/// A list, and how many it holds.
pub fn counted(key: &str, items: Vec<Value>) -> Value {
    json!({ "count": items.len(), key: items })
}

/// The file the user has open, and where the caret stands, counted from one.
pub fn open_file(
    path: &str,
    worktree: Option<&WorktreeId>,
    commit: Option<&str>,
    at: (usize, usize),
    unsaved: bool,
) -> Value {
    json!({ "open_file": {
        "path": path,
        "worktree_id": worktree,
        "commit": commit,
        "line": at.0 + 1,
        "column": at.1 + 1,
        "unsaved": unsaved,
    }})
}

/// No file open.
pub fn no_file() -> Value {
    json!({ "open_file": null })
}

pub fn file(path: &str, commit: Option<&str>, text: &str) -> Value {
    json!({ "path": path, "commit": commit, "text": text })
}

pub fn annotations(notes: &[Annotation]) -> Value {
    json!({ "annotations": notes })
}

pub fn mr(stored: Option<&Mr>) -> Value {
    json!({ "mr": stored })
}

pub fn threads(url: &str, threads: &[MrThread]) -> Value {
    json!({ "mr": url, "threads": threads })
}

pub fn ci(url: &str, run: Option<&CiStatus>) -> Value {
    json!({ "mr": url, "ci": run })
}
