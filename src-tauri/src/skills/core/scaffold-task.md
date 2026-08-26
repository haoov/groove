---
name: scaffold-task
description: Read the task and attach the repos and branch it needs.
groove-kinds: task
groove-label: scaffold
---

# Set the task up to work in

1. `get_task_body` and `get_active_task` — what the task asks for, and what it
   already has.
2. `list_repos` — the exact names. `add_task_repo` cannot clone, so a repo missing
   from this list has to be cloned by the user first.
3. `add_task_repo` for each repo the task needs and does not have.

Propose the list in the chat before the first call, with one line saying why each
repo is on it. Say plainly when the body names no repo — guessing from the title
attaches the wrong one.
