---
name: save-task
description: Land the work on a task — commit, push, open or update the MR on every worktree that has work, and bring the task itself up to date. Use when the user says to save the work, ship it, or to commit and push and update the ticket.
groove-kinds: task
groove-label: save task
groove-hint: Commit, push, MR, and update the task.
---

# Land the work

A task can hold several repos, and several worktrees on one repo. Steps 3 to 5
run once for EVERY worktree that has work, not for the one that has the most.
Steps 6 to 8 run once for the task.

1. `get_active_task`, `get_commit_log` and `get_task_diff` — every worktree the
   task has, and what each one carries.
2. Name the worktrees that have work and what each will get, in one line, then
   go. Work through them one at a time and finish a worktree before starting the
   next, so an approval always belongs to the branch you just named.

Per worktree with work:

3. `git_commit`, when it has uncommitted changes. Each worktree gets its own
   message, describing that repo's change rather than the task as a whole.
4. `git_push`, when the branch is ahead of origin.
5. `get_mr_state` for that worktree. No MR, `create_mr`. An open MR that no
   longer describes the work, `update_mr`. An MR that still reads true, leave it.

Then once, for the task:

6. `get_task_body`, then `update_task_body` with the WHOLE new body — it replaces
   the page. Cover every repo you touched. Keep what the user wrote and add the
   outcome, not a narration of the work.
7. `update_task_property` for a status the work has moved past. Name the property
   exactly as the task's source spells it.
8. `get_task_time`, then `log_task_hours` with its `unlogged_hours`, when that is
   above zero. That figure is what Groove measured and has not yet sent, so it is
   the whole of what to log. Never round it up, pad it, or invent one from the
   commits — and never log `tracked_hours`, which double-counts what is already
   there.

A worktree with nothing to do is skipped, not reported as a failure.

When one worktree fails or is refused, land the others that do not depend on it,
then stop and say exactly which worktrees are short and what each still needs.
Where the repos have to move together, stop instead — a change the user believes
is shipped and is only half there is the worst outcome of this skill.
