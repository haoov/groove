//! MCP tool schemas advertised by `tools/list`.
//!
//! `inputSchema` is camelCase per the MCP spec.
//!
//! THIS FILE IS THE ONE PLACE that tells the agent how to write a commit message,
//! MR text, an annotation or a task body. It is the only text present at the moment
//! of the call, attached to the field it governs, and it reaches every caller — a
//! canned prompt only fires for a pill button. Do NOT restate these rules in
//! src/lib/prompts.ts or in a CLAUDE.md: saying the same thing in three places got
//! all three ignored.
//!
//! Keep every rule to a short phrase. No examples, no rationale, no length caps —
//! the agent needs scoping, not an essay.
//!
//! The PreToolUse hook (~/.claude/hooks/conventional-check.py) DENIES a call that
//! breaks the hard ones: the subject grammar, the annotation label, the MR's
//! What/Why headings. Change a hard rule here and change it there in the same edit.

/// Commit-subject grammar. Used by git_commit and both MR titles.
const SUBJECT: &str = "Conventional commit subject: `type(scope): subject`, imperative, \
    lower case, no final period, under 72 chars. Types: feat, fix, chore, docs, style, \
    refactor, perf, test, build, ci, revert.";

/// The bar for length. A deletion test, never a number — a sentence budget produces
/// clipped prose instead of short prose.
const TIGHT: &str = "Only what the reader needs to act on. Cut what adds nothing.";

/// Markdown affordances. Stated because the bans on file lists and per-commit
/// changelogs otherwise read as "prose only". `- [ ]` becomes a real to-do
/// block (task_manager/notion.rs) and a GitLab task list.
const LISTS: &str = "Lists where you are listing things. `- [ ]` for anything still open.";

fn mr_description() -> String {
    format!(
        "Markdown. Required: `## What`, then `## Why`. Why ends with how you verified \
         it. Further headings when the change needs them. {TIGHT} Nothing the diff \
         already shows: no file list, no per-commit changelog. {LISTS} The task \
         link is appended automatically."
    )
}

/// Body rules for a task being DRAFTED. Not used by update_task_body, which replaces
/// a page the user may have written by hand.
fn task_body_description(specifics: &str) -> String {
    format!(
        "Markdown under the template's headings. {TIGHT} {specifics} {LISTS} Checkboxes \
         become real to-do items."
    )
}

const TARGET_BRANCH: &str = "Branch this work is based on and will be merged back into. Omit for the repo default; name it for a maintenance, release or backport branch, or for the branch a stacked MR sits on. It must already exist on origin. The branch is cut from it, the diff is measured against it, and create_mr targets it.";

pub(crate) fn mcp_tool_definitions() -> Vec<serde_json::Value> {
    vec![
        mcp_tool("get_active_task", "Get the currently open task, its worktrees, and repos.", serde_json::json!({"type":"object","properties":{}})),
        mcp_tool("list_tasks", "Every real task the app knows about, with status and priority. Use it to answer what is queued or in progress.", serde_json::json!({"type":"object","properties":{}})),
        mcp_tool("list_repos", "Repos cloned under MAIN, each flagged `attached` when it is already on your task. Use it to get the exact name for add_task_repo.", serde_json::json!({"type":"object","properties":{}})),
        mcp_tool("get_worktrees", "List worktrees for the active task.", serde_json::json!({"type":"object","properties":{"task_id":{"type":"string"}}})),
        mcp_tool("get_task_diff", "Diff for all worktrees of a task (vs origin/main; for explorer sessions: the uncommitted working-tree changes).", serde_json::json!({"type":"object","required":["task_id"],"properties":{"task_id":{"type":"string"}}})),
        mcp_tool("get_commit_log", "Commit history for all worktrees of a task.", serde_json::json!({"type":"object","required":["task_id"],"properties":{"task_id":{"type":"string"},"limit":{"type":"integer"}}})),
        mcp_tool("get_mr_state", "MR/PR state for a worktree.", serde_json::json!({"type":"object","required":["worktree_id"],"properties":{"worktree_id":{"type":"string"}}})),
        mcp_tool("get_annotations", "All annotations for a task.", serde_json::json!({"type":"object","required":["task_id"],"properties":{"task_id":{"type":"string"}}})),
        mcp_tool("get_open_file", "Currently open file in the editor.", serde_json::json!({"type":"object","properties":{}})),
        mcp_tool("get_file_content", "Read file content by path.", serde_json::json!({"type":"object","required":["file_path"],"properties":{"file_path":{"type":"string"}}})),
        mcp_tool("get_task_body", "Fetch a task's body as markdown.", serde_json::json!({"type":"object","properties":{"task_id":{"type":"string","description":"Defaults to your own task."}}})),
        mcp_tool("git_commit", "Stage all and commit. Requires user confirmation.", serde_json::json!({"type":"object","required":["worktree_id","message"],"properties":{"worktree_id":{"type":"string"},"message":{"type":"string","description":format!("{SUBJECT} Body optional: why, not what. No file lists.")}}})),
        mcp_tool("git_push", "Push branch to origin. Requires confirmation.", serde_json::json!({"type":"object","required":["worktree_id"],"properties":{"worktree_id":{"type":"string"}}})),
        mcp_tool("git_pull", "Pull --rebase from origin. Requires confirmation.", serde_json::json!({"type":"object","required":["worktree_id"],"properties":{"worktree_id":{"type":"string"}}})),
        mcp_tool("git_rebase", "Rebase on origin/main. Requires confirmation.", serde_json::json!({"type":"object","required":["worktree_id"],"properties":{"worktree_id":{"type":"string"},"default_branch":{"type":"string"}}})),
        mcp_tool("create_mr", "Create MR/PR. Requires confirmation.", serde_json::json!({"type":"object","required":["worktree_id","title"],"properties":{"worktree_id":{"type":"string"},"title":{"type":"string","description":SUBJECT},"description":{"type":"string","description":mr_description()}}})),
        mcp_tool("update_mr", "Update MR title/description. Requires confirmation.", serde_json::json!({"type":"object","required":["mr_id"],"properties":{"mr_id":{"type":"string","description":"The MR's id from get_mr_state or the create reply. Its number (!42) also works when only one MR has it."},"title":{"type":"string","description":SUBJECT},"description":{"type":"string","description":mr_description()}}})),
        mcp_tool("add_task_repo", "Attach a repo to your task and create its worktree. Requires confirmation. The repo must already be cloned under MAIN — call list_repos first; this tool cannot clone.", serde_json::json!({"type":"object","required":["repo"],"properties":{"repo":{"type":"string","description":"Slug (group/path/project) or project name from list_repos."},"task_id":{"type":"string","description":"Defaults to your own task."},"branch":{"type":"string","description":"Branch for the new worktree. Defaults to the task branch."},"target_branch":{"type":"string","description":TARGET_BRANCH}}})),
        mcp_tool("add_task_worktree", "Check out ANOTHER branch of a repo your task already has, as a second worktree beside the first. Requires confirmation. Use add_task_repo instead to attach a repo the task does not have yet.", serde_json::json!({"type":"object","required":["branch"],"properties":{"branch":{"type":"string","description":"Branch for the new worktree. Must differ from the ones already checked out; include the task id so it stays traceable."},"repo":{"type":"string","description":"Project name from get_worktrees. Only needed when the task has more than one repo."},"task_id":{"type":"string","description":"Defaults to your own task."},"target_branch":{"type":"string","description":TARGET_BRANCH}}})),
        mcp_tool("close_mr", "Close MR without merging. Requires confirmation.", serde_json::json!({"type":"object","required":["mr_id"],"properties":{"mr_id":{"type":"string","description":"The MR's id from get_mr_state or the create reply. Its number (!42) also works when only one MR has it."}}})),
        mcp_tool("create_annotation", "Annotate a line (or line range) in a file (no confirmation needed). Use line_num for a single line, or start_line+end_line for a multiline range.", serde_json::json!({"type":"object","required":["task_id","repo_id","file_path","content"],"properties":{"task_id":{"type":"string"},"repo_id":{"type":"string"},"file_path":{"type":"string"},"line_num":{"type":"integer"},"start_line":{"type":"integer"},"end_line":{"type":"integer"},"content":{"type":"string","description":"Conventional Comment: `label:` or `label (decoration):`, then the problem in plain language. Labels: issue, suggestion, nitpick, question, todo, praise, thought, typo, polish, quibble, note, chore. Decorations: blocking, non-blocking, if-minor. Short. Do not restate the code. Markdown renders, so `code` for identifiers. A [claude] marker is added for you; do not write it."},"author":{"type":"string"}}})),
        mcp_tool("update_annotation", "Rewrite an annotation's body, yours or the user's (no confirmation needed). The line range and the author stay as they are. Use it to correct a note you already left, not to add a new one.", serde_json::json!({"type":"object","required":["id","content"],"properties":{"id":{"type":"string"},"content":{"type":"string","description":"The replacement note, in full. Same rules as create_annotation: Conventional Comment, short, markdown."}}})),
        mcp_tool("resolve_annotation", "Mark annotation as resolved (no confirmation).", serde_json::json!({"type":"object","required":["id"],"properties":{"id":{"type":"string"}}})),
        mcp_tool("create_task", "File a NEW task. It is not opened or checked out — it lands in the queue. Call get_task_template first and mirror its headings.", serde_json::json!({"type":"object","required":["title"],"properties":{"title":{"type":"string","description":"One line naming the outcome, in plain language. NOT a conventional-commit subject — no \"type(scope):\" prefix."},"body_markdown":{"type":"string","description":task_body_description("Drop headings with nothing real to say.")},"provider":{"type":"string","description":format!("{}. Only needed when more than one is set up.", crate::provider::names_prose())},"repo":{"type":"string","description":"owner/repo, for a source that files per repo."}}})),
        mcp_tool("update_task_property", "Set one property of a task (Priority, Platform Components, Tags, Due, …). Requires confirmation. Call get_active_task for the task id; the value shape follows the property type: select/status/url/date = string, number = number, checkbox = bool, multi_select = array of option names, relation = array of page ids.", serde_json::json!({"type":"object","required":["property","value"],"properties":{"task_id":{"type":"string","description":"Defaults to your own task."},"property":{"type":"string","description":"Exact property name, as the task's source spells it."},"value":{"description":"Shape depends on the property type; null clears it."}}})),
        mcp_tool("log_task_hours", "ADD hours to the task's \"Hours spent\" (never replaces it). Requires confirmation. Only log time the user asked you to log.", serde_json::json!({"type":"object","required":["hours"],"properties":{"task_id":{"type":"string","description":"Defaults to your own task."},"hours":{"type":"number","description":"Hours to add, e.g. 1.5."}}})),
        mcp_tool("update_task_body", "Replace the task page's body with markdown. Requires confirmation. Read the current body with get_task_body first and send the WHOLE new body — this replaces, it does not append. Fails if the page holds blocks markdown can't rebuild.", serde_json::json!({"type":"object","required":["markdown"],"properties":{"task_id":{"type":"string","description":"Defaults to your own task."},"markdown":{"type":"string","description":format!("The complete new body — this REPLACES the page. Keep what the user wrote unless asked to change it. {LISTS}")},"force":{"type":"boolean","description":"Only after the user accepts losing unsupported blocks."}}})),
        mcp_tool("get_task_template", "Fetch the task template as markdown (template_markdown), empty when the source has none. Mirror its headings when drafting body_markdown for create_task_from_explorer — but keep each section to a line or two.", serde_json::json!({"type":"object","properties":{"provider":{"type":"string","description":format!("{}. Only needed when more than one source is set up and the session has no task.", crate::provider::names_prose())}}})),
        mcp_tool("create_task_from_explorer", "Draft and file a task from the current explorer session, then convert the session into a task (requires user confirmation). First call get_task_template and mirror its headings. Provide a title and a SHORT markdown body.", serde_json::json!({"type":"object","required":["title","body_markdown"],"properties":{"title":{"type":"string","description":"One line naming the outcome, not the investigation, in plain language. NOT a conventional-commit subject — no \"type(scope):\" prefix."},"body_markdown":{"type":"string","description":task_body_description("Drop headings with nothing real to say. No narration of the exploration.")},"provider":{"type":"string","description":format!("{}. Only needed when more than one is set up.", crate::provider::names_prose())}}})),
    ]
}

fn mcp_tool(name: &str, description: &str, input_schema: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

#[cfg(test)]
mod tests {
    use super::mcp_tool_definitions;

    /// The hook denies a description without these, so the contract must name them.
    #[test]
    fn mr_description_states_the_required_headings() {
        let tools = mcp_tool_definitions();
        for name in ["create_mr", "update_mr"] {
            let tool = tools.iter().find(|t| t["name"] == name).expect(name);
            let d = tool["inputSchema"]["properties"]["description"]["description"]
                .as_str()
                .expect("description text");
            assert!(d.contains("## What"), "{name}: no What heading");
            assert!(d.contains("## Why"), "{name}: no Why heading");
            assert!(d.contains("- [ ]"), "{name}: no checkbox affordance");
            // The cap that produced staccato prose must not come back.
            assert!(!d.contains("maximum"), "{name}: reintroduced a length cap");
            assert!(!d.contains("sentences"), "{name}: reintroduced a sentence budget");
        }
    }

    #[test]
    fn drafted_task_bodies_ask_for_checkboxes() {
        let tools = mcp_tool_definitions();
        for name in ["create_task", "create_task_from_explorer"] {
            let tool = tools.iter().find(|t| t["name"] == name).expect(name);
            let d = tool["inputSchema"]["properties"]["body_markdown"]["description"]
                .as_str()
                .expect("body text");
            assert!(d.contains("- [ ]"), "{name}: no checkbox affordance");
            assert!(!d.contains("maximum"), "{name}: reintroduced a length cap");
        }
    }
}

