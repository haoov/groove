---
name: fix-notes
description: Fix the open notes on this session — the MR's discussion threads. Use when the user asks you to address the notes, the comments or the review feedback on this session, or to fix what a reviewer flagged.
groove-kinds: task, review
groove-label: fix notes
groove-hint: Fix the open notes on this session.
---

# Fix the open notes

1. `get_active_task`, then `get_mr_state` for each worktree. Where there is an
   MR, `get_mr_threads` — each unresolved thread is a note, and a CI bot's
   comment says what the pipeline saw.
2. Fix the ones that are real problems, in the worktree the thread points at.
3. Name each thread you fixed in the chat, and what to answer on it. Do not
   reply or resolve it yourself unless the user asks.

Skip a note you disagree with and say why in one line. Answer a note that asks
a question in the chat.
