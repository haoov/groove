---
name: open-mr
description: Open a merge request for the work on this session's branch. Use when the user asks to open, raise or submit an MR, a PR or a pull request for what is on the branch.
groove-kinds: task
groove-label: open MR
groove-hint: Open an MR for this branch.
---

# Open an MR

1. `get_active_task` — the task's worktrees, each with its repo and branch.
2. `get_task_diff` and `get_commit_log` — what changed, and which worktrees carry
   commits. Read them before writing anything: the branch name is not the change.
3. `get_mr_state` on the worktree you picked — one that already has an open MR
   needs `update_mr`, not a second MR.
4. `create_mr`.

Pick the worktree by what has work, not by what is on screen.
