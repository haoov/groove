---
name: co-review
description: Summarize an MR and annotate the real problems in it. Use when the user asks you to review this MR or PR, look over someone else's change, do a code review, or say what is wrong with it.
groove-kinds: review
groove-label: co-review
groove-hint: Review this MR and note what is wrong.
---

# Co-review this MR

1. `get_task_diff` — the whole change.
2. Say in the chat what the MR does and why, with context on the components it
   touches.
3. `create_annotation` where something is actually wrong: bugs, broken edge cases,
   security problems, breaking changes.

Skip style nits and anything you would phrase as "consider…". A review with three
real findings beats one with thirty.
