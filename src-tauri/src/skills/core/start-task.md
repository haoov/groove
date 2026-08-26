---
name: start-task
description: Orient on this task before writing code — read it, survey what exists, propose a plan.
groove-kinds: task
groove-label: start task
---

# Orient before writing code

Read-only, and it stops before implementing: end with a plan and wait.

1. `get_active_task` — the repos, worktrees and branches you have.
2. `get_task_body` — the ask, and any acceptance criteria. Say plainly when the
   body is thin. Do not invent requirements it does not state.
3. `get_task_diff`, `get_commit_log` and `get_annotations` — the branches may
   already carry work, so do not assume a blank slate. Tell real commits from
   dependency-bot noise, and treat an unresolved annotation as part of the job.
4. Read each worktree's `CLAUDE.md` or `AGENTS.md`. The plan has to fit them.

Then summarize, skimmable: the goal, what each worktree already holds, the open
annotations, and an ordered plan naming the repo for every step.
