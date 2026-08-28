---
name: new-skill
description: Write or edit one of the user's own agent skills. Use when the user asks for a new skill, action or button, wants one of theirs changed or renamed, or says to turn something into a skill.
groove-kinds: task, review, explorer
groove-label: new skill
groove-hint: Draft a skill the user asks for.
---

# Write a skill for the user

The USER says what it should do. Draft it from what they asked for, never from
what this session happens to be doing.

1. Ask what the skill should do, if they have not said already. Ask again only
   for what you cannot pick sensibly yourself — what should trigger it, and which
   sessions it belongs to.
2. `read_user_skill` when they are changing one of theirs. The save replaces the
   whole file, so you need all of it.
3. `save_user_skill`.

The confirmation shows them the file; do not paste it into the chat as well. Say
plainly when it lands that it loads on the agent's next start — the action bar
offers the reload.
