# Groove

Groove is a desktop workspace for a platform engineer who works through agents. One
window holds [sessions](glossary.md#session) — a task, a review or an exploration — each
with its repos, worktrees, diffs, MRs, terminals and one agent. The user opens sessions,
watches what the agents do, approves what leaves the machine, reviews the changes and
lands them.

Groove takes each capability of the classic tools — tracker, git client, editor,
terminal, forge — and organises it around sessions, worktrees and the agent instead of
around files, windows or pages.

## What Groove answers

- Centralises the work of a platform engineer into one window, one identity.
- Makes the agent's work visible, reviewable and reversible.
- Keeps the work of a task together, across providers, repos, worktrees, branches and MRs.
- Brings code review into the tool.

## What the docs hold

The docs hold the intent, the rules and the decisions. The code holds what exists.

| Question | Where the answer is |
|---|---|
| Why is it built this way? What must never happen? | these docs |
| Which crates exist, and what does each hold? | each crate's `description` in its `Cargo.toml` and its `//!` doc |
| Which actions exist? | the `Command` enum of each controller, one doc comment a variant |
| What does each service own? | the `//!` doc of the service crate |
| What is planned? | the GitHub issues |

A doc never lists what the code already lists. A doc that names a file, a function or a
test names one that exists: `docs.rs` in the controllers' tests checks it.

## The capabilities

Five, and they do not move. Growth is answered inside them.

| Capability | Answers | Surface |
|---|---|---|
| [Tasks](capabilities/tasks.md) | pull your tasks from their providers and work them as sessions | board |
| [Agent](capabilities/agent.md) | run one agent per session, see what each does, approve what it writes | rail |
| [Sessions](capabilities/sessions.md) | keep a task's repos, worktrees and agent together | session: overview and agent pane |
| [Workspace](capabilities/workspace.md) | review, edit and land the code | session: work region |
| [Config](capabilities/config.md) | set Groove up and keep its preferences | settings |

## Files

| File | Read it for |
|---|---|
| [glossary.md](glossary.md) | the words every other doc uses, one line each |
| [architecture.md](architecture.md) | the layers, their rules, state and threads, rendering, tests |
| [trace.md](trace.md) | one action, `workspace.push`, followed from the click to the feed |
| [design.md](design.md) | the surfaces and the rules they follow |
| [capabilities/](capabilities/) | one file per capability: its tool sets and the rules each follows |
| [shared.md](shared.md) | the skills, knowledge and routines a team shares |
