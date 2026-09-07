//! One argument struct per tool, mirroring its `inputSchema` in `definitions.rs`.
//! Every tool parses its arguments once here; a failure is JSON-RPC invalid params.

use serde::{Deserialize, Serialize};

/// Marks an argument error so `handle_jsonrpc` can answer -32602.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub(crate) struct InvalidParams(pub(crate) String);

/// Marks a tool name the dispatcher has no arm for; also -32602.
#[derive(Debug, thiserror::Error)]
#[error("Unknown tool: {0}")]
pub(crate) struct UnknownTool(pub(crate) String);

// ─── Parsing ──────────────────────────────────────────────────────────────────

pub(super) trait ToolArgs: serde::de::DeserializeOwned {
    /// Ranges and combinations `inputSchema` cannot express.
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

/// Deserialize a tool's arguments, then range-check them.
pub(super) fn parse<T: ToolArgs>(input: serde_json::Value) -> anyhow::Result<T> {
    // An argument-less call sends no `arguments` key at all.
    let input = if input.is_null() {
        serde_json::json!({})
    } else {
        input
    };
    let args: T = serde_json::from_value(input).map_err(invalid)?;
    args.validate().map_err(invalid)?;
    Ok(args)
}

/// Parse, validate, then re-serialize as the confirmation-bridge payload.
pub(super) fn payload<T: ToolArgs + Serialize>(
    input: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    Ok(serde_json::to_value(parse::<T>(input)?)?)
}

fn invalid(e: impl std::fmt::Display) -> anyhow::Error {
    anyhow::Error::new(InvalidParams(e.to_string()))
}

fn non_empty(field: &str, value: &str) -> Result<(), String> {
    match value.trim().is_empty() {
        true => Err(format!("{field} must not be empty")),
        false => Ok(()),
    }
}

// ─── Bounded scalars ──────────────────────────────────────────────────────────

/// A 1-based line number.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(try_from = "i64")]
pub(super) struct Line(i64);

impl Line {
    pub(super) fn get(self) -> i64 {
        self.0
    }
}

impl TryFrom<i64> for Line {
    type Error = String;
    fn try_from(v: i64) -> Result<Self, String> {
        match v >= 1 {
            true => Ok(Self(v)),
            false => Err(format!("line numbers start at 1, got {v}")),
        }
    }
}

/// A commit-log page size.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(try_from = "u64")]
pub(super) struct Limit(u32);

impl Limit {
    const MAX: u64 = 1000;
    pub(super) fn get(self) -> u32 {
        self.0
    }
}

impl TryFrom<u64> for Limit {
    type Error = String;
    fn try_from(v: u64) -> Result<Self, String> {
        match (1..=Self::MAX).contains(&v) {
            true => Ok(Self(v as u32)),
            false => Err(format!(
                "limit must be between 1 and {}, got {v}",
                Self::MAX
            )),
        }
    }
}

// ─── Reads ────────────────────────────────────────────────────────────────────

/// Tools whose schema declares no properties. Deliberately permissive: with no
/// optional field to misspell, refusing a stray key only breaks working calls.
#[derive(Debug, Deserialize)]
pub(super) struct NoArgs {}

impl ToolArgs for NoArgs {}

/// `task_id` alone: every read and write that defaults to the caller's own task.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaskArgs {
    pub(super) task_id: Option<String>,
}

impl ToolArgs for TaskArgs {
    fn validate(&self) -> Result<(), String> {
        opt_id("task_id", &self.task_id)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CommitLogArgs {
    pub(super) task_id: Option<String>,
    pub(super) limit: Option<Limit>,
}

impl ToolArgs for CommitLogArgs {
    fn validate(&self) -> Result<(), String> {
        opt_id("task_id", &self.task_id)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RelationArgs {
    pub(super) property: String,
    pub(super) task_id: Option<String>,
}

impl ToolArgs for RelationArgs {
    fn validate(&self) -> Result<(), String> {
        non_empty("property", &self.property)?;
        opt_id("task_id", &self.task_id)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TemplateArgs {
    pub(super) provider: Option<String>,
}

impl ToolArgs for TemplateArgs {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SkillNameArgs {
    pub(super) name: String,
}

impl ToolArgs for SkillNameArgs {
    fn validate(&self) -> Result<(), String> {
        non_empty("name", &self.name)
    }
}

// ─── Bridged git and MR writes ────────────────────────────────────────────────

/// `git_push`, `git_pull` and `get_mr_state`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WorktreeArgs {
    pub(super) worktree_id: String,
}

impl ToolArgs for WorktreeArgs {
    fn validate(&self) -> Result<(), String> {
        non_empty("worktree_id", &self.worktree_id)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CommitArgs {
    pub(super) worktree_id: String,
    pub(super) message: String,
}

impl ToolArgs for CommitArgs {
    fn validate(&self) -> Result<(), String> {
        non_empty("worktree_id", &self.worktree_id)?;
        non_empty("message", &self.message)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RebaseArgs {
    pub(super) worktree_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) default_branch: Option<String>,
}

impl ToolArgs for RebaseArgs {
    fn validate(&self) -> Result<(), String> {
        non_empty("worktree_id", &self.worktree_id)?;
        opt_id("default_branch", &self.default_branch)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CreateMrArgs {
    pub(super) worktree_id: String,
    pub(super) title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) description: Option<String>,
}

impl ToolArgs for CreateMrArgs {
    fn validate(&self) -> Result<(), String> {
        non_empty("worktree_id", &self.worktree_id)?;
        non_empty("title", &self.title)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateMrArgs {
    pub(super) mr_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) description: Option<String>,
}

impl ToolArgs for UpdateMrArgs {
    fn validate(&self) -> Result<(), String> {
        non_empty("mr_id", &self.mr_id)?;
        match (&self.title, &self.description) {
            (None, None) => Err("nothing to update: pass title, description, or both".into()),
            _ => Ok(()),
        }
    }
}

/// `close_mr`, `get_mr_ci` and `get_mr_threads`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MrArgs {
    pub(super) mr_id: String,
}

impl ToolArgs for MrArgs {
    fn validate(&self) -> Result<(), String> {
        non_empty("mr_id", &self.mr_id)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SaveSkillArgs {
    pub(super) name: String,
    pub(super) body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) previous: Option<String>,
}

impl ToolArgs for SaveSkillArgs {
    fn validate(&self) -> Result<(), String> {
        non_empty("name", &self.name)?;
        non_empty("body", &self.body)
    }
}

// ─── Task writes ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CreateTaskArgs {
    pub(super) title: String,
    pub(super) body_markdown: Option<String>,
    pub(super) provider: Option<String>,
    pub(super) repo: Option<String>,
}

impl ToolArgs for CreateTaskArgs {
    fn validate(&self) -> Result<(), String> {
        non_empty("title", &self.title)
    }
}

/// `repo` is accepted but not advertised; it defaults to the explorer's first repo.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FromExplorerArgs {
    pub(super) title: String,
    pub(super) body_markdown: String,
    pub(super) provider: Option<String>,
    pub(super) repo: Option<String>,
}

impl ToolArgs for FromExplorerArgs {
    fn validate(&self) -> Result<(), String> {
        non_empty("title", &self.title)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AddRepoArgs {
    pub(super) repo: String,
    pub(super) task_id: Option<String>,
    pub(super) branch: Option<String>,
    pub(super) target_branch: Option<String>,
}

impl ToolArgs for AddRepoArgs {
    fn validate(&self) -> Result<(), String> {
        non_empty("repo", &self.repo)?;
        opt_id("task_id", &self.task_id)?;
        opt_id("target_branch", &self.target_branch)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AddWorktreeArgs {
    pub(super) branch: String,
    pub(super) repo: Option<String>,
    pub(super) task_id: Option<String>,
    pub(super) target_branch: Option<String>,
}

impl ToolArgs for AddWorktreeArgs {
    fn validate(&self) -> Result<(), String> {
        non_empty("branch", &self.branch)?;
        opt_id("task_id", &self.task_id)?;
        opt_id("target_branch", &self.target_branch)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PropertyArgs {
    pub(super) task_id: Option<String>,
    pub(super) property: String,
    pub(super) value: serde_json::Value,
}

impl ToolArgs for PropertyArgs {
    fn validate(&self) -> Result<(), String> {
        non_empty("property", &self.property)?;
        opt_id("task_id", &self.task_id)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HoursArgs {
    pub(super) task_id: Option<String>,
    pub(super) hours: f64,
}

impl ToolArgs for HoursArgs {
    fn validate(&self) -> Result<(), String> {
        if !self.hours.is_finite() || self.hours <= 0.0 {
            return Err(format!(
                "hours must be a positive number, got {}",
                self.hours
            ));
        }
        opt_id("task_id", &self.task_id)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaskBodyArgs {
    pub(super) task_id: Option<String>,
    pub(super) markdown: String,
    #[serde(default)]
    pub(super) force: bool,
}

impl ToolArgs for TaskBodyArgs {
    fn validate(&self) -> Result<(), String> {
        opt_id("task_id", &self.task_id)
    }
}

// ─── Annotations ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CreateAnnotationArgs {
    pub(super) task_id: String,
    pub(super) repo_id: String,
    pub(super) file_path: String,
    pub(super) line_num: Option<Line>,
    pub(super) start_line: Option<Line>,
    pub(super) end_line: Option<Line>,
    pub(super) content: String,
    pub(super) author: Option<String>,
}

impl CreateAnnotationArgs {
    /// The annotated span; `end_line` defaults to the start. The store orders the pair.
    pub(super) fn range(&self) -> (i64, i64) {
        let start = self.start_line.or(self.line_num).map_or(1, Line::get);
        (start, self.end_line.map_or(start, Line::get))
    }
}

impl ToolArgs for CreateAnnotationArgs {
    fn validate(&self) -> Result<(), String> {
        non_empty("task_id", &self.task_id)?;
        non_empty("repo_id", &self.repo_id)?;
        non_empty("file_path", &self.file_path)?;
        non_empty("content", &self.content)?;
        match (self.line_num, self.start_line) {
            (None, None) => Err("line_num, or start_line with end_line, is required".into()),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateAnnotationArgs {
    pub(super) id: String,
    pub(super) content: String,
}

impl ToolArgs for UpdateAnnotationArgs {
    fn validate(&self) -> Result<(), String> {
        non_empty("id", &self.id)?;
        non_empty("content", &self.content)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AnnotationArgs {
    pub(super) id: String,
}

impl ToolArgs for AnnotationArgs {
    fn validate(&self) -> Result<(), String> {
        non_empty("id", &self.id)
    }
}

fn opt_id(field: &str, value: &Option<String>) -> Result<(), String> {
    match value {
        Some(v) => non_empty(field, v),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err<T: ToolArgs + std::fmt::Debug>(input: serde_json::Value) -> String {
        parse::<T>(input).expect_err("must be refused").to_string()
    }

    #[test]
    fn an_absent_argument_object_is_an_empty_one() {
        assert!(parse::<NoArgs>(serde_json::Value::Null).is_ok());
        assert!(parse::<TaskArgs>(serde_json::Value::Null).is_ok());
    }

    #[test]
    fn a_line_number_starts_at_one() {
        let base = serde_json::json!({
            "task_id": "t", "repo_id": "r", "file_path": "a.rs", "content": "note: x",
        });
        for bad in [0, -1, -400] {
            let mut input = base.clone();
            input["line_num"] = serde_json::json!(bad);
            assert!(err::<CreateAnnotationArgs>(input).contains("line numbers start at 1"));
        }

        let mut ok = base.clone();
        ok["line_num"] = serde_json::json!(12);
        assert_eq!(parse::<CreateAnnotationArgs>(ok).unwrap().range(), (12, 12));

        let mut range = base.clone();
        range["start_line"] = serde_json::json!(4);
        range["end_line"] = serde_json::json!(9);
        assert_eq!(
            parse::<CreateAnnotationArgs>(range).unwrap().range(),
            (4, 9)
        );

        assert!(err::<CreateAnnotationArgs>(base).contains("is required"));
    }

    #[test]
    fn a_limit_is_bounded_and_never_truncated() {
        // `as u32` used to wrap this to 0.
        let big = serde_json::json!({ "limit": u64::from(u32::MAX) + 1 });
        assert!(err::<CommitLogArgs>(big).contains("limit must be between 1 and 1000"));
        assert!(err::<CommitLogArgs>(serde_json::json!({ "limit": 0 })).contains("between"));
        assert_eq!(
            parse::<CommitLogArgs>(serde_json::json!({ "limit": 50 }))
                .unwrap()
                .limit
                .unwrap()
                .get(),
            50
        );
    }

    #[test]
    fn a_misspelled_optional_argument_is_refused() {
        let input = serde_json::json!({ "worktree_id": "w-1", "defaultBranch": "release" });
        assert!(err::<RebaseArgs>(input).contains("unknown field"));
    }

    #[test]
    fn an_empty_identifier_is_not_an_identifier() {
        assert!(
            err::<WorktreeArgs>(serde_json::json!({ "worktree_id": " " }))
                .contains("worktree_id must not be empty")
        );
        assert!(err::<AnnotationArgs>(serde_json::json!({ "id": "" })).contains("must not be"));
        assert!(err::<TaskArgs>(serde_json::json!({ "task_id": "" })).contains("must not be"));
    }

    #[test]
    fn hours_must_be_positive_and_finite() {
        for bad in [0.0, -2.5] {
            let input = serde_json::json!({ "hours": bad });
            assert!(err::<HoursArgs>(input).contains("positive number"));
        }
        assert_eq!(
            parse::<HoursArgs>(serde_json::json!({ "hours": 1.5 }))
                .unwrap()
                .hours,
            1.5
        );
    }

    #[test]
    fn an_mr_update_that_changes_nothing_is_refused() {
        let input = serde_json::json!({ "mr_id": "m-1" });
        assert!(err::<UpdateMrArgs>(input).contains("nothing to update"));
    }

    #[test]
    fn a_bridge_payload_keeps_only_what_was_sent() {
        let payload = payload::<UpdateMrArgs>(serde_json::json!({
            "mr_id": "m-1", "title": "fix(mcp): type the arguments",
        }))
        .unwrap();
        assert_eq!(payload["mr_id"], "m-1");
        assert_eq!(payload["title"], "fix(mcp): type the arguments");
        assert!(
            payload.get("description").is_none(),
            "an absent description must stay absent, not become an empty body"
        );
    }
}
