//! Confirmation-bridge op_type names — single source of truth.
//!
//! These identify a queued write op across three places that must agree: the
//! `bridge.post(...)` call site, the `execute_op` dispatch match, and the
//! frontend confirmation UI. Mirror in `src/lib/ops.ts`.

pub const GIT_COMMIT: &str = "git.commit";
pub const GIT_PUSH: &str = "git.push";
pub const GIT_PULL: &str = "git.pull";
pub const GIT_REBASE: &str = "git.rebase";
pub const GIT_DISCARD: &str = "git.discard";
pub const GIT_DISCARD_ALL: &str = "git.discard_all";

pub const MR_CREATE: &str = "mr.create";
pub const MR_UPDATE: &str = "mr.update";
pub const MR_CLOSE: &str = "mr.close";

pub const NOTION_STATUS: &str = "notion.status";
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
