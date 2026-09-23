---
name: fix-notes
description: Fix the open notes on this session — the annotations and the MR's discussion threads. Use when the user asks you to address the notes, the comments or the review feedback on this session, or to fix what a reviewer flagged.
groove-kinds: task, review
groove-label: fix notes
groove-hint: Fix the open notes on this session.
---

# Fix the open notes

1. `get_annotations` — the notes on this session, from the user and from you.
2. `get_active_task`, then `get_mr_state` for each worktree. Where there is an
   MR, `get_mr_threads` — the unresolved threads are notes too, and a CI bot's
   comment says what the pipeline saw.
3. Fix the ones that are real problems, in the worktree the note points at.
4. `resolve_annotation` for each annotation you actually fixed. An MR thread you
   fixed stays open: name it in the chat, the user resolves it on the forge.

Skip a note you disagree with and say why in one line. Never resolve a note you
did not fix. Answer a note that asks a question in the chat.
