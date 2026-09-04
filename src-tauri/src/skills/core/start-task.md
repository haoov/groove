---
name: start-task
description: Orient on a task before writing code: read it, attach the repos it needs, survey what the branches already hold, propose a plan. Use when the user asks where to start, what the plan is, to set the task up, or to look at the task before touching anything.
groove-kinds: task
groove-label: start task
groove-hint: Read the task and propose a plan.
---

# Orient before writing code

It reads and plans, and stops before implementing: end with a plan and wait.
Attaching the repos (step 3) is the one thing it starts.

1. `get_active_task` — the repos, worktrees and branches you have.
2. `get_task_body` — the ask, and any acceptance criteria. Say plainly when the
   body is thin. Do not invent requirements it does not state.
3. With no repos attached, add them: `list_repos` for the exact names, then
   `add_task_repo` for each one the body names. Propose the list first, a line
   each on why it is on it. Say plainly when the body names no repo — guessing
   from the title attaches the wrong one — and when a repo is missing from
   `list_repos`, which means the user has to clone it first.
4. `get_task_diff`, `get_commit_log` and `get_annotations` — the branches may
   already carry work, so do not assume a blank slate. Tell real commits from
   dependency-bot noise, and treat an unresolved annotation as part of the job.
   Skip this on a branch you created a moment ago; it has no history.
5. Read each worktree's `CLAUDE.md` or `AGENTS.md`. The plan has to fit them.

Then summarize, skimmable: the goal, what each worktree already holds, the open
annotations, and an ordered plan naming the repo for every step.
