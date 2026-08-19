//! The op catalog: every approval-gated write's name AND its executor, side by
//! side. Adding an op is one constant plus one `execute` arm here — the frontend
//! renders it by the same name (`src/lib/ops.ts`, checked by the mirror test).

use sqlx::SqlitePool;
use tauri::AppHandle;

pub const GIT_COMMIT: &str = "git.commit";
pub const GIT_PUSH: &str = "git.push";
pub const GIT_PULL: &str = "git.pull";
pub const GIT_REBASE: &str = "git.rebase";
pub const GIT_DISCARD: &str = "git.discard";
pub const GIT_DISCARD_ALL: &str = "git.discard_all";

pub const MR_CREATE: &str = "mr.create";
pub const MR_UPDATE: &str = "mr.update";
pub const MR_CLOSE: &str = "mr.close";

/// Set any editable Notion property (agent-initiated; the UI writes directly).
pub const NOTION_PROPERTY: &str = "notion.property";
/// Add hours to the task's "Hours spent" number.
pub const NOTION_HOURS: &str = "notion.hours";
/// Replace the task page's body from markdown — gated even from the UI, because
/// it can delete blocks markdown cannot represent.
pub const NOTION_BODY: &str = "notion.body";

/// File a new task in Notion without opening or provisioning anything.
pub const TASK_CREATE: &str = "task.create";
/// Attach an already-cloned repo to a task and provision its worktree.
pub const TASK_ADD_REPO: &str = "task.add_repo";
pub const TASK_CREATE_FROM_EXPLORER: &str = "task.create_from_explorer";

/// Every op, for the mirror test below.
#[cfg(test)]
const ALL: [&str; 15] = [
    GIT_COMMIT,
    GIT_PUSH,
    GIT_PULL,
    GIT_REBASE,
    GIT_DISCARD,
    GIT_DISCARD_ALL,
    MR_CREATE,
    MR_UPDATE,
    MR_CLOSE,
    NOTION_PROPERTY,
    NOTION_HOURS,
    NOTION_BODY,
    TASK_CREATE,
    TASK_ADD_REPO,
    TASK_CREATE_FROM_EXPLORER,
];

/// The success payload every write op returns. A bare `null` is what an agent
/// gets when a tool call does nothing, so ops that "just succeed" must still say
/// so explicitly — otherwise the model reads success as failure and retries.
fn op_ok(op: &str, message: impl Into<String>) -> serde_json::Value {
    serde_json::json!({ "ok": true, "op": op, "message": message.into() })
}

/// Repo name for readable messages: the last segment of the worktree path.
fn repo_of(payload: &serde_json::Value) -> String {
    payload["worktree_path"]
        .as_str()
        .unwrap_or("")
        .rsplit('/')
        .find(|s| !s.is_empty())
        .unwrap_or("the repo")
        .to_string()
}

fn branch_of(payload: &serde_json::Value) -> String {
    payload["branch"].as_str().unwrap_or("HEAD").to_string()
}

/// Dispatch an approved write op to its implementation. Returns op-specific metadata.
pub(super) async fn execute(
    op_type: &str,
    payload: serde_json::Value,
    pool: &SqlitePool,
    handle: &AppHandle,
) -> anyhow::Result<serde_json::Value> {
    match op_type {
        GIT_COMMIT => {
            let (repo, branch) = (repo_of(&payload), branch_of(&payload));
            let subject = payload["message"]
                .as_str()
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("")
                .to_string();
            crate::worktrees::commit_impl(payload, pool).await?;
            Ok(op_ok(op_type, format!("Committed \"{subject}\" on {branch} in {repo}")))
        }
        GIT_PUSH => {
            let (repo, branch) = (repo_of(&payload), branch_of(&payload));
            crate::worktrees::push_impl(payload).await?;
            Ok(op_ok(op_type, format!("Pushed {branch} to origin in {repo}")))
        }
        GIT_PULL => {
            let (repo, branch) = (repo_of(&payload), branch_of(&payload));
            crate::worktrees::pull_impl(payload).await?;
            Ok(op_ok(op_type, format!("Pulled origin/{branch} into {repo}")))
        }
        GIT_REBASE => {
            let (repo, branch) = (repo_of(&payload), branch_of(&payload));
            // rebase_impl already reports status/files (the UI keys on
            // `status == "conflict"`); enrich it rather than replace it.
            let mut v = crate::worktrees::rebase_impl(payload).await?;
            let conflicted = v["status"].as_str() == Some("conflict");
            let files = v["files"].as_array().map(|a| a.len()).unwrap_or(0);
            if let Some(obj) = v.as_object_mut() {
                obj.insert("ok".into(), serde_json::json!(true));
                obj.insert("op".into(), serde_json::json!(op_type));
                obj.insert(
                    "message".into(),
                    serde_json::json!(if conflicted {
                        format!("Rebase of {branch} stopped on conflicts in {files} file(s) — resolve them, then continue the rebase")
                    } else {
                        format!("Rebased {branch} onto its base branch in {repo}")
                    }),
                );
            }
            Ok(v)
        }
        GIT_DISCARD => {
            let repo = repo_of(&payload);
            let file = payload["file_path"].as_str().unwrap_or("the file").to_string();
            crate::worktrees::discard_impl(payload).await?;
            Ok(op_ok(op_type, format!("Discarded local changes in {file} ({repo})")))
        }
        GIT_DISCARD_ALL => {
            let repo = repo_of(&payload);
            crate::worktrees::discard_all_impl(payload).await?;
            Ok(op_ok(op_type, format!("Discarded ALL local changes in {repo}")))
        }
        MR_CREATE => {
            let branch = branch_of(&payload);
            let worktree_id = payload["worktree_id"].as_str().unwrap_or("").to_string();
            crate::forge::create_mr_impl(payload, pool).await?;
            // Hand back the MR the op just recorded — the agent's next step is
            // almost always "give me the link".
            let latest = crate::core::db::store::mrs::latest_for_worktree(pool, &worktree_id)
                .await
                .ok()
                .flatten();
            Ok(match latest {
                Some(mr) => serde_json::json!({
                    "ok": true,
                    "op": op_type,
                    "message": format!("Merge request !{} created from {branch}", mr.remote_id),
                    "iid": mr.remote_id,
                    "url": mr.url,
                }),
                None => op_ok(op_type, format!("Merge request created from {branch}")),
            })
        }
        MR_UPDATE => {
            crate::forge::update_mr_impl(payload, pool).await?;
            Ok(op_ok(op_type, "Merge request title/description updated"))
        }
        MR_CLOSE => {
            crate::forge::close_mr_impl(payload, pool).await?;
            Ok(op_ok(op_type, "Merge request closed"))
        }
        NOTION_PROPERTY => {
            let out = crate::notion::update_property_impl(payload, pool).await?;
            let prop = out["property"].as_str().unwrap_or("property").to_string();
            let value = out["value"].as_str().unwrap_or("").to_string();
            Ok(op_ok(op_type, if value.is_empty() {
                format!("Cleared {prop}")
            } else {
                format!("{prop} set to \"{value}\"")
            }))
        }
        NOTION_HOURS => {
            let out = crate::task_manager::log_hours_impl(payload, pool).await?;
            let (before, after) =
                (out["before"].as_f64().unwrap_or(0.0), out["after"].as_f64().unwrap_or(0.0));
            Ok(op_ok(op_type, format!("Hours spent {before} → {after}")))
        }
        NOTION_BODY => {
            // Carries its own message (block counts).
            crate::notion::update_body_impl(payload, pool).await
        }
        TASK_CREATE => crate::notion::create_task_impl(payload, pool).await,
        TASK_ADD_REPO => {
            // Local git and DB work only. The handle is for the workspace_ready
            // refresh the add ends with.
            crate::task_manager::add_repo_impl(payload, pool, handle).await
        }
        TASK_CREATE_FROM_EXPLORER => {
            // Already returns the created task (short_id, page id, …).
            crate::task_manager::create_task_from_explorer_impl(payload, pool).await
        }
        _ => Err(anyhow::anyhow!("unknown op_type: {op_type}")),
    }
}

#[cfg(test)]
mod tests {
    use super::ALL;

    /// The frontend renders each op by name from its own mirror. A backend op
    /// missing there is a raw-JSON dialog — silent, so it fails here instead.
    /// (The reverse direction — dead frontend names — is checked once the
    /// frontend rework lands; ops.ts still carries known-dead entries.)
    #[test]
    fn every_op_exists_in_the_frontend_mirror() {
        let ts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/lib/ops.ts");
        let ts = std::fs::read_to_string(&ts).expect("src/lib/ops.ts must exist");
        for op in ALL {
            assert!(ts.contains(&format!("'{op}'")), "op {op} missing from src/lib/ops.ts");
        }
    }

    #[test]
    fn every_op_name_is_unique() {
        let mut seen = std::collections::HashSet::new();
        for op in ALL {
            assert!(seen.insert(op), "duplicate op name: {op}");
        }
    }
}
