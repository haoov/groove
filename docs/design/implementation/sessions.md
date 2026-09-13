# Sessions — implementation

How the tool sets in [../capabilities/sessions.md](../capabilities/sessions.md) are
made: today, from the code; planned, in the words of
[../architecture.md](../architecture.md).

Kinds: task, explorer, review. `sessions.kind` is the discriminator; one provisioning
path serves all three.

## Repos management

### Today

The clone pool lives at `<worktree root>/main/<host>/<group>/<project>` and is listed
by a directory walk. `register_repo` records a clone in the database after one git
call checks it has an `origin`. `clone_repo` clones a URL into the pool. A repo is
named by its whole slug or any unique suffix — `host/a/b`, `a/b`, `b` — so an agent's
short name resolves. A review session takes no extra repo. `add_repo_impl` attaches
a repo to a session, provisions its worktree and refreshes the workspace; it is the
bridge path for `task.add_repo`.

### Planned

**Module `worktree`**, on `git` and `db`: the pool walk, register, clone, and the
name resolution. **Service `session`** owns the repo list per session in its slice.

| Controller | Does |
|---|---|
| `session.add_repo` | resolve the name, register or clone, then `session.add_worktree` for the session's default branch |
| `session.remove_repo` | close its worktrees, detach the repo from the session |

A clone is never written to. The pool is refreshed by `git fetch` before a worktree is
cut from it.

## Worktrees management

### Today

A worktree lives at `<worktree root>/worktrees/<session id>/<project>/<branch>`, the
branch keeping its slashes as directories. The default branch is
`<type>/<slug>-<id>` — the type guessed from the title's words, the slug from the
title, the id the task's tag at its source — and `explorer/<slug>` for explorers.
`validate_branch_name` refuses what `git check-ref-format` would. Provisioning
fetches the clone, creates the branch when needed, adds the worktree and records it;
it is idempotent on an existing worktree for the same branch. A local branch that
exists and does not name the session is refused unless the caller named it; an
adopted branch is reported. A new branch is cut from the chosen source, else the
repo's default; the target branch is checked against `origin` and the refusal lists
the ones that exist. A review worktree checks out the MR's source branch with its
target as `base_ref`. `close_worktree` removes the directory and prunes the clone's
registration; the branch stays. `cleanup_session_worktrees` removes every worktree of
a session and the session directory.

### Planned

**Module `worktree`**: naming, validation, provisioning, teardown, exactly as today.
**Service `session`** keeps per session the worktrees, the selected one, and each
worktree's `WorktreeDelivery` from `forge` and `git`.

| Controller | Does |
|---|---|
| `session.add_worktree` | validate, fetch, provision from source with target as `base_ref`, record; refresh the workspace |
| `session.select_worktree` | set the session's selected worktree; diff, editor, terminals and commit box follow |
| `session.close_worktree` | teardown the directory, prune, leave the branch |

The pickers for source and target list `origin`'s heads from `worktree`.

## Overview

### Today

Task overview: body editor writing to the source through the bridge, property strip
and header pickers for status and size, time fields, finish, delete, sync, and the
`start-task` chip on a task with no repos. Explorer overview: rename, discard,
convert — file the task at its provider, then move worktrees, rows and the agent
session onto the new id. Review overview: the checked-out MR with a Co-review
button. `open_task` opens a session; `set_active_task` points the backend at the
focused one; `finish_task` sets the source status to done and tears the workspace
down; `delete_task` discards at the source and tears down locally.

### Planned

One overview for the three kinds, drawn from the `session` slice, in the order of
[../design.md](../design.md): properties, repos with their worktree rows, body.

| Controller | Does |
|---|---|
| `session.open` | create or load the session, provision what is missing, `agent.start`, set the task in progress, select the last-touched worktree, add the row to the rail |
| `session.close` | `agent.end`, remove the row; the session stays on disk |
| `session.select` | make it the current one; clear its *unseen* |
| `session.open_explorer` · `session.rename_explorer` · `session.discard_explorer` · `session.convert_explorer` | as today; convert moves rows and the agent onto the new id |
| `session.open_review` | register the MR's clone, provision the review worktree, `agent.start` |
| `session.get_active` · `session.get` | reads: the active session; one session with its repos and worktrees |
| `session.list_repos` · `session.list_branches` | reads: the pool; `origin`'s heads for the pickers |

The worktree row's delivery icons come from `WorktreeDelivery`; its skill button sends
`agent.send_skill`. Property and body edits, finish and delete are `task.*` controllers; the overview
only renders them. Finish is offered when every worktree is merged or closed.

## Agent session

### Today

An xterm console on the agent's PTY, base64 both ways through `write_pty` and
`pty_output`. `start_agent_session` spawns it; `sendSkill` writes `/groove:<name>`
and Enter; `reloadAgent` stops and starts with `--resume`. The Actions drop-up lists
the skills the agent has; a write to a skill marks the list stale until reload. The
`autoApprove` toggle sits in the console header.

### Planned

**The pane** draws the agent's `terminal` grid; keys go to the PTY as bytes. Escape
reaches the TUI like any key. The action bar: the skills menu, reload, and the ask
with Approve and Review.

| Controller | Does |
|---|---|
| `agent.send` | write bytes to the PTY |
| `agent.send_skill` | write `/groove:<name>` and Enter |
| `agent.reload` | end, then start with `--resume` |

**Scoped skills**: `--plugin-dir` per launch, so the pane's menu lists exactly what
this agent has; stale after a save until reload.

## Needs

- [ ] `session_state`: one leaf row per session — opened_at, seen_at, auto_approve, selected_worktree_id — so `sessions` is never altered again.
- [ ] `WorktreeDelivery` in `types` and its fold in the `session` service.
- [ ] The last-touched worktree per session, from the timeline.
- [ ] Explorer conversion as one controller over `task`, `session` and `agent`.
