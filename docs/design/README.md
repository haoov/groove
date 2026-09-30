# Groove

Groove is a desktop workspace for a platform engineer who works through agents. One
window holds sessions — a task, a review or an exploration — each with its repos,
worktrees, diffs, MRs, terminals and one agent. The user opens sessions, watches what
the agents do, approves what leaves the machine, reviews the changes and lands them.

Groove is the one tool for that work. It takes each capability of the classic tools
— tracker, git client, editor, terminal, forge — and organises it around sessions,
worktrees and the agent instead of around files, windows or pages.

## What Groove answers

- Centralises the work of a platform engineer into one window, one identity.
- Makes the agent's work visible, reviewable and reversible.
- Keeps the work of a task together, across providers, repos, worktrees, branches and MRs.
- Brings code review into the tool.

## Files

Each file describes one level and names only the level below it. The README names
capabilities. A capability names its tool sets. A tool set names its commands. The docs
hold the decisions and the rules; the code holds how they are carried out.

| File | Describes |
|---|---|
| [capabilities.md](capabilities.md) | the five capabilities and their tool sets |
| [capabilities/](capabilities/) | one file per capability: its tool sets and what each manages |
| [implementation/](implementation/) | one file per capability: the decisions each tool set is made of, and what is left to build |
| [architecture.md](architecture.md) | layers, boundaries, splitting, state ownership, rendering, tests, migration |
| [homes.md](homes.md) | the services, and the one home of each concept |
| [design.md](design.md) | the surfaces — board, rail, session, review sheet, settings — and the rules they follow |
| [maquette.html](maquette.html) | the surfaces as a working page; open it in a browser |
| [clusters.md](clusters.md) | the clusters a session works on: designed, not built |
