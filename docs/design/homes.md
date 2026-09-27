# Homes

Every concept has one home. Whatever needs it calls that home and never derives it again.

## The three layers

- **Controller**: the gate between an event and an action. It calls the services the action
  touches and routes their answers into state. It holds no logic beyond that grouping.
- **Service**: owns one part of the app. It holds that part's state and carries out its
  actions through modules.
- **Module**: does the work: git, the forge, the database, the parser, the PTY.

## Services

Seven services, one part of the app each. `workspace` splits in two: the code in a worktree,
and what the forge says about it.

| Service | Owns | Modules |
|---|---|---|
| `session` | the sessions, their repos and worktrees, each worktree's git status, the rail, the log | `sessions`, `worktree`, `git`, `timeline` |
| `task` | the tasks and their sources, the plan, the clock and hours, attention, where a new task is filed | `provider`, `plan`, `ledger` |
| `workspace` | the selected worktree's change: files, rows, colours, each worktree's open buffers, search, paths, history, the index, commit, push, pull | `diff`, `text`, `editor`, `grep`, `git`, `watch` |
| `delivery` | each worktree's MR: state, CI, threads, the session's notes, verdicts, the poll, the review queue | `forge`, `mrs`, `annotations` |
| `agent` | each session's agent: terminal, activity, asks, launch files, skills | `terminal`, `agent-launch`, `approvals`, `skills`, `hooks`, `mcp`, `tools` |
| `shell` | each session's own terminals: its tabs, the terminals side by side in each, the one taking the keys | `terminal` |
| `config` | the config file, preferences, settings | `config` |

## One home per concept

| Concept | Module | Service | Replaces |
|---|---|---|---|
| A worktree's MR, CI and threads | `forge` reads, `mrs` stores | `delivery`: one map keyed by worktree | `workspace.delivery`, the mr/ci fields of session rows, `workspace.facts` |
| Whether a CI run has finished; what an MR or CI change is worth on the log | — | `delivery` | `workspace/mr.rs` `finished`, `became`, `moved` |
| Whether an MR can be opened, written or closed | — | `delivery`, for the worktree named | `workspace/write.rs` `allows` (read the selected one) |
| When the poll reads which worktree | — | `delivery` | `workspace/mr.rs` `wanted`, `first`, `again` |
| The review queue | `forge` | `delivery` | `workspace.reviews`, `workspace/queue.rs` |
| A session's notes, and notes with threads as one list | `annotations` | `delivery`, per session | `workspace.own`, `workspace.notes` (selected session only) |
| One open note per line | `annotations` | — | `workspace/notes.rs` `noted` |
| A note action; a thread action | — | `delivery`: two enums | one `Act` with catch-alls |
| The forge client for a repo | `forge` | inside `delivery` | `Service::remote` built in five controllers |
| The base branch of a worktree | `git` `base_ref` | — | `read.rs` `ORIGIN_HEAD`, `commits.rs` literal, `unpushed` fallback |
| Commits origin lacks | `git` | — | worktree `status`, teardown `unpushed`, workspace `unpushed` |
| A worktree's git status | `worktree` | `session` | — |
| The selected worktree | — | `session` state | 9 helpers in controllers |
| A worktree by id, in whichever session holds it | — | `session` state | `pair`, `worktree_named`, `stood`, `moved`, `onto` |
| Opening a session, row and repo in order | `sessions`, `worktree` | `session`: one call | three controller copies; the review FK race |
| Deleting a session | `worktree`, `sessions` | `session` | — |
| A line on a session's log, and the feed | `timeline` | `session`: append and fold into the feed | `Services.timeline` in controllers, three `logged` helpers |
| Attention: start dates, MR facts per task, the fold | — | `task` | `task/attention.rs` |
| Whether a task is being worked | — | `task` timer | `task/time.rs` `worked` |
| Where a new task is filed | — | `task` | `tools/filing.rs` |
| Which rows get their colours read | `diff` | `workspace` | `workspace/diff.rs` `show` window |
| A gap's hidden lines | `diff` | `workspace` | `workspace/diff.rs` `gap_at` |
| Whether a re-read keeps or replaces the buffer | `text` | `workspace` | `workspace/editor.rs` `arrived` |
| An answer that still fits the selection | — | each service: its reads carry what they were read for | unchecked in `load`, `read`, `commits` |
| Launch files and the conversation handoff | `agent-launch` | `agent` | — |
| The terminal of a session | `terminal` | `agent` state | 8 copies of the lookup |
| What a tool answers, and an ask's text | `tools` | — | JSON built in `controllers/tools`, push text in a controller |
| A database failure, and a store on memory | `db` | — | four copies filed as `Invalid` |
| A list that scrolls: its extent, clip and culling | — | `ui` widget `scrolled` | eight copies in the views |
| Where a scroller keeps its offset | — | `Ui::offset`, `Ui::wheeled` | five `moved` calls in the wheel input |
| A module's failure | the module | — | `agent-launch` enum of one variant |

## Errors

A module keeps its own error enum only when its variants map to different kinds, or a caller
matches one. Every other module returns `groove_types::Error`.

## Controllers after the move

A controller function reads its arguments, calls services, puts the answers into state and
calls the next controller. Nothing else.

| Controller | Keeps | Moved out |
|---|---|---|
| `session` | open, select, close, delete, repos: service calls in order | the open sequence and delete order → `session`; job helpers → `spawn` |
| `task` | load, open, finish, adopt, log hours, status | attention and the worked rule → `task` |
| `workspace` | one function per command, calling `workspace` | the prefetch window, gaps, buffer rules → `workspace`; MR, notes, queue → `delivery` |
| `delivery` (new) | poll tick, MR writes, notes, verdicts | — |
| `agent` | start, end, send, pointer, skills | the terminal lookup → `agent` state |
| `tools` | one function per tool: call the controller or service, reply | JSON shapes → `tools`; filing → `task` |
