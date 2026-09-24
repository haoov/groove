---
name: create-task
description: File a new task at its source — a follow-up, a side task, or what this session found. Use when the user asks to file, open or create a task, ticket or issue.
groove-label: create task
groove-hint: File a new task at its source.
---

# File a task

An argument, when given, is the source to file at — `notion`, `github`.

1. `get_task_template` — the headings to mirror, and `file_at`: the source, and on
   Notion the database and the status and assignee a new page is given.
2. File the task yourself, at that source. On Notion, a page in
   `file_at.database_id` through the Notion MCP, with the status and the assignee
   `file_at` names. On GitHub, `gh issue create` in this session's repo. Groove
   never writes the task.
3. Say the task's URL. This session goes on with what it was doing.

The title names the outcome, not the investigation. Drop any heading you have
nothing real to say under.

When the Notion MCP is not connected in this session, stop and tell the user. Do
not file the task some other way.
