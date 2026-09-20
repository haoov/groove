# Sessions — implementation

How the tool sets in [../capabilities/sessions.md](../capabilities/sessions.md) are made,
in the words of [../architecture.md](../architecture.md). Each table lists the actions
still to build; for the rest the `Command` enum is the truth.

Kinds: task, explorer, review. `sessions.kind` is the discriminator; one provisioning
path serves all three.

## Repos management

**Module `worktree`**, on `git` and `db`: the pool walk, register, clone, and the
name resolution. **Service `session`** owns the repo list per session in its slice.

A clone is never written to. The pool is refreshed by `git fetch` before a worktree is
cut from it.

## Worktrees management

**Module `worktree`**: naming, validation, provisioning, teardown. **Service `session`**
keeps per session the worktrees, the selected one, and each worktree's `WorktreeDelivery`
from `forge` and `git`.

The pickers for source and target list `origin`'s heads from `worktree`.

## Overview

One overview for the three kinds, drawn from the `session` slice, in the order of
[../design.md](../design.md): properties, repos with their worktree rows, body.

**Closing is not deleting.** A closed session keeps its worktrees, its repos and its
row; it leaves the rail and stays on the board's Live column, which lists every session
on disk. Picking it there puts it back on the rail with its agent.
Only `session.delete` takes it away.

| Still to build | Does |
|---|---|
| `session.discard_explorer` · `session.convert_explorer` | discard; or file the task and move the worktrees, the rows and the agent onto the new id |
| `session.open_review` | register the MR's clone, provision the review worktree, `agent.start` |
| `session.get_active` · `session.get` | reads for the MCP tools |

The worktree row's delivery icons come from `WorktreeDelivery`; its skill button sends
`agent.send_skill`. Property and body edits, finish and delete are `task.*` controllers;
the overview only renders them. Finish is offered when every worktree is merged or closed.

## Agent session

**The pane** draws the agent's `terminal` grid; keys go to the pty as bytes. Escape
reaches the TUI like any key. The action bar: the skills menu, reload, and the ask
with Approve and Review.

| Still to build | Does |
|---|---|
| `agent.send_skill` | write `/groove:<name>` and Enter |
| `agent.reload` | end, then start with `--resume` |

**Scoped skills**: `--plugin-dir` per launch, so the pane's menu lists exactly what
this agent has; stale after a save until reload.

## Needs

- [x] `session_state`: one leaf row per session, so `sessions` is never altered again.
- [x] `WorktreeDelivery` in `types` and its fold in the `session` service.
- [ ] The last-touched worktree per session, from the timeline.
- [ ] Explorer conversion as one controller over `task`, `session` and `agent`.
