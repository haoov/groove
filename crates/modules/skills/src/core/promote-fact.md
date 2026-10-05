---
name: promote-fact
description: Add a fact to the team's shared knowledge — from this conversation or from your own memory — checked against its source first. Use when the user asks to share, promote or record a fact for the team, or says the others should know it.
groove-label: share a fact
groove-hint: Add a fact to what the team knows.
---

# Share a fact with the team

1. Name the fact: from the conversation, or from your own memory when the user
   points at one. Ask only when it is not clear which.
2. Refuse what is about one person: how the user works, what they prefer, what
   they told you to do. Those stay in your own memory. Say why in one line.
3. Check it now against its source: read the file, the commit or the ticket it
   comes from. A fact the source no longer bears out is not shared; say what
   changed instead. A fact with no source you can read is not shared either.
4. Read the team's `INDEX.md`. A fact already there is updated in place, not
   written twice.
5. Open an MR on the shared repo — the project the shared copy's `origin` names.
   It adds `knowledge/<name>.md`, with the header that repo's `README.md` shows,
   and the fact's line in `knowledge/INDEX.md`. Never write in the shared copy.
6. Say the MR's link, and the fact with its source.
