---
name: update-task
description: Bring the task's body and properties up to date with the work done. Use when the user asks to update the task, the ticket or the issue, or to write up on it what was done.
groove-kinds: task
groove-label: update task
groove-hint: Update the task with what was done.
---

# Update the task

1. `get_task_body` — the current body. `update_task_body` REPLACES it, so you need
   all of it.
2. `get_task_diff` and `get_commit_log` — what actually happened.
3. `update_task_body` with the whole new body, and `update_task_property` for a
   property the work changed.

Keep what the user wrote unless they asked you to change it. Add the outcome, not
a narration of the work.
