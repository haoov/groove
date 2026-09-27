//! How the agent writes what a human reads; the only place these rules are stated.

/// Commit-subject grammar, for a commit and for both MR titles.
pub const SUBJECT: &str = "Conventional commit subject: `type(scope): subject`, imperative, \
    lower case, no final period, under 72 chars. Types: feat, fix, chore, docs, style, \
    refactor, perf, test, build, ci, revert.";

/// The length rule: a deletion test, not a number.
pub const TIGHT: &str = "Only what the reader needs to act on. Cut what adds nothing.";

/// Markdown affordances; `- [ ]` becomes a real to-do block.
pub const LISTS: &str = "Lists where you are listing things. `- [ ]` for anything still open.";

/// What a note on a line says, and how.
pub const NOTE: &str = "Conventional Comment: `label:` or `label (decoration):`, then the \
    problem in plain language. Labels: issue, suggestion, nitpick, question, todo, praise, \
    thought, typo, polish, quibble, note, chore. Decorations: blocking, non-blocking, \
    if-minor. Short. Do not restate the code. Markdown renders, so `code` for identifiers.";

/// Which branch a worktree is cut from and measured against.
pub const TARGET_BRANCH: &str = "Branch this work is based on and will be merged back into. \
    Omit for the repo default; name it for a maintenance, release or backport branch, or \
    for the branch a stacked MR sits on. It must already exist on origin. The branch is cut \
    from it, the diff is measured against it, and create_mr targets it.";

/// The body of a merge request.
pub fn mr_description() -> String {
    format!(
        "Markdown. Required: `## What`, then `## Why`. Why ends with how you verified it. \
         Further headings when the change needs them. {TIGHT} Nothing the diff already \
         shows: no file list, no per-commit changelog. {LISTS} The task link is appended \
         automatically."
    )
}

/// What a skill's own file holds.
pub const SKILL: &str = "Front matter first: name, description, and the groove- keys \
     (label, hint, kinds). Then the steps, in the imperative. It says WHAT to do; how to \
     word a commit or a note belongs to the tool that takes it.";
