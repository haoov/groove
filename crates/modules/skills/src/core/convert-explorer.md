---
name: convert-explorer
description: File a task from this exploration and make this session the one that works it. Use when the user asks to turn the exploration into a task, to convert it, or to start working on what it found as a task.
groove-kinds: explorer
groove-label: convert to task
groove-hint: File a task and work it here.
---

# Turn this exploration into a task

An argument, when given, is the source to file at — `notion`, `github`.

1. File the task the way `groove:create-task` does — the template, then the task
   at its source.
2. `adopt_task` with the URL the source gave back. This session then works that
   task, with its repos and its worktrees as they are.
