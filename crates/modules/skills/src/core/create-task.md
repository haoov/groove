---
name: create-task
description: File a new task at its source — a follow-up, a side task, or what this session found. Use when the user asks to file, open or create a task, ticket or issue.
groove-label: create task
groove-hint: File a new task at its source.
---

# File a task

An argument, when given, is the source to file at — `notion`, `github`.

1. `get_task_template`, with `provider` set to the argument when one is given. It
   returns the headings to mirror, and `file_at`: the source, and on Notion the
   database and the status and assignee a new page is given. When it says several
   sources are set up and no argument names one, ask the user which.
2. Read the source's schema: on Notion, the database `file_at.database_id` through
   the Notion MCP; on GitHub, the fields of the project board the repo's issues are on.
3. File the task yourself, at that source, with every property you can fill. On Notion, the sprint
   is the sprint running now: read the sprint relation's own database for the current one.
   Set the priority, the dates, the estimate, the project and any other property from
   what the conversation and the task say. Leave a property empty only when nothing
   says what it holds. On GitHub, `gh issue create` in this session's repo, then add
   it to the board and set its fields the same way. Groove never writes the task.
4. Say the task's URL and the properties you set. This session goes on with what it
   was doing.

The title names the outcome, not the investigation. Drop any heading you have
nothing real to say under.

When the Notion MCP is not connected in this session, stop and tell the user. Do
not file the task some other way.
