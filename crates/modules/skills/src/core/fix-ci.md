---
name: fix-ci
description: Find why this branch's pipeline failed and fix it. Use when the user says CI is red, the pipeline or the build is failing, the checks are broken, or asks you to fix CI.
groove-kinds: task
groove-label: fix CI
groove-hint: Read the failing job and fix it.
---

# Fix the pipeline

1. `get_active_task`, then `get_mr_state` per worktree — the MR whose pipeline is
   red. Say so and stop when no worktree has one.
2. `get_mr_ci` for that MR — the status and the run's URL.
3. Read the failing job's log with the forge's own CLI: `glab ci` on GitLab,
   `gh run` on GitHub. The log names the failure; the status does not.
4. Reproduce it locally in the worktree, with the repo's own commands. Its
   `CLAUDE.md` or `AGENTS.md` names them — a green local run of a different
   command proves nothing.
5. Fix it, then run that same command again until it passes.

A failure the log blames on the environment — a runner, a registry, a timeout,
a flake — is not a code fix. Say which it is and stop rather than editing around
it.

Leave the change uncommitted and say what you did. `groove:save-task` lands it
when the user wants it landed.
