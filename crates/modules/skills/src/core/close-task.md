---
name: close-task
description: End a task - check nothing is unlanded, write it up, then mark it done and tear the workspace down. Use when the user says the task is finished, done, or asks to close it, wrap it up for good, or clear it away.
groove-kinds: task
groove-label: close task
groove-hint: Check it is landed, then close it.
---

# Close the task

`finish_task` deletes every worktree of this session. Steps 1 to 3 are what makes
that safe, so do them first and report what they found. Never call `finish_task`
on a check you have not run.

1. `get_active_task`, then `get_task_diff` for every worktree — anything
   uncommitted is work about to be destroyed.
2. `get_commit_log` and `get_mr_state` per worktree — a branch ahead of origin,
   or an MR still open, means the work has not landed anywhere but this machine.
3. `get_annotations` — an unresolved note is a job not finished.
4. Stop and say so when any of those find something. Offer `groove:save-task`
   to land it first. Closing anyway is the user's call, never yours.
5. Write it up — skip this entirely when `groove:save-task` just ran, because its
   last steps are these. `get_task_body` then `update_task_body` with the WHOLE
   new body, saying what shipped. `get_task_time` then `log_task_hours` with its
   `unlogged_hours` when that is above zero.
6. `finish_task`. It sets the status done at the source itself, so no
   `update_task_property` for that.

Say what the checks found either way, before the last call. The user approves
`finish_task` on what you tell them.
