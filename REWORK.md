# Rework ledger

The running record of the core rework. Every phase notes what it changed, what
it deliberately broke, and what later phases must pick up. Read the pending
list for your component before touching it.

Target structure: `core/` (db, git, config, pty, http, timing) + feature
modules (worktrees, review, sessions, approvals, notion, forge, agent), and a
feature-sliced frontend. One rule everywhere: shared code is promoted into
`core/`, never copied.

---

## Phase 1 — core/db: store layer + sessions-first schema (done)

### Schema (migration 0006, final — edited in place while unreleased)

Tables: `sessions`, `notion_tasks`, `repos`, `session_repos`, `worktrees`,
`mrs`, `annotations`, `time_entries`, `time_logs`, `pending_confirmations`.

- `tasks` split: `sessions` = local state (kind/state columns), `notion_tasks`
  = pure queue mirror, truncatable and re-synced by `list_tasks`.
- Session kinds: `task | explorer | review`. **The desk is gone** (see 1b).
- `sessions.state (open|paused)` replaces `worktrees.is_active`.
- Review identity = `(review_project, review_iid)` UNIQUE — ids are derived
  `review-<project-slug>-<iid>`, no more hashed ids.
- Foreign keys ON; every session child cascades on DELETE and UPDATE. Teardown
  is one `DELETE FROM sessions`; explorer→task conversion re-keys the session
  row and children follow.
- Worktrees: `UNIQUE(session_id, repo_id, branch)` — **multiple worktrees per
  repo per session are legal**; git's own one-branch-one-worktree rule is the
  real constraint. `path` UNIQUE.
- `mrs`: `UNIQUE(worktree_id, remote_id)`, platform CHECK.
- Time is a ledger: `time_entries(session_id, day, seconds)` +
  `time_logs` (every Notion write, append-only). Unlogged =
  `MAX(0, Σentries − Σlogs)` at read time.
- Dropped tables: `tab_snapshots`, `agent_sessions` (was write-only; PTY truth
  is in-memory), `reviewed_files` (feature removed end to end).
- Pool: WAL, `synchronous=NORMAL`, `busy_timeout(5s)`, foreign_keys on.

### Store layer

`core/db/store/{sessions,notion_tasks,repos,worktrees,mrs,annotations,time,
confirmations,home}.rs` is the ONLY place SQL exists — enforced by the
`no_raw_sql_outside_the_store` test. Functions take `impl SqliteExecutor` so
they compose into transactions. Errors are `StoreError` (entity + id), mapped
to `String` once at the IPC edge.

Transacted flows: `worktrees::close` (delete + detach repo when last),
`sessions::adopt_explorer` (mirror upsert + session re-key + worktree
branch/path updates).

### Renames that ripple

- `task_id` → `session_id` on: `Worktree`, `Annotation`,
  `PendingConfirmation`, the `worktree_closed` / `confirmation_*` event
  payloads, `create_annotation` / `get_annotations` IPC args, `TaskTime`.
  MCP tool *inputs* still say `task_id` (external agent contract); outputs
  carry `session_id`.
- `git_engine::task_dir` → `session_dir`; `cleanup_task_worktrees` →
  `cleanup_session_worktrees`.
- The `TaskView` DTO keeps the old frontend `Task` shape
  (`short_id`/`notion_page_id`/`status`/…) as a strangler boundary — the
  frontend rename to Session happens in the frontend phase.

### Removed

`db/load.rs`, `delete_task_rows` + `OWNED_TABLES`, hand-written cascades,
`is_active` filters (9 frontend sites), kind prefix-sniffing (4 sites),
`migrate_layout.rs` (one-shot, already ran everywhere it ever would),
reviewed-files feature (commands, Home count, viewed-✓ UI, store slice).

### Added

`core/timing`: `timed(op, detail, fut)` logs ≥2ms operations under
`RUST_LOG=timing=debug`. Wired into `run_git_output`, `glab_run`, `gh_run`,
Notion HTTP. Later phases wrap new subprocess/HTTP call sites the same way.

### Bugs closed here

B4 (Notion pagination), B13 (confirmations follow conversion via cascade),
B15 (userinfo stripped in `parse_git_url`), N4 (review id collisions),
N10 (eslint error), SQLITE_BUSY + crash-half-state classes.

---

## Phase 1b — DB additions: multi-worktree, no desk (done)

Decisions taken with the schema still unreleased, so 0006 was edited in place
(never ran on a real DB).

- **Multi-worktree**: `worktrees` unique key is `(session_id, repo_id,
  branch)`. `store::worktrees::upsert` keys on the triple;
  `for_repo(session, repo)` returns `Vec<Worktree>`.
- **Desk removed** from the schema (kind CHECK, partial index, seed) and from
  the backend (`DESK_ID`, `ensure_desk_session`, provision guard arm,
  `SessionKind::Desk`). `sessions::remove` is now unconditional.

### Deliberately broken, waiting on later phases

| Surface | State | Owning phase |
|---|---|---|
| Home agent console + Home terminal | `ensure_desk_session` no longer exists; the desk-bound UI in `App.tsx`, `lib/desk.ts`, `prompts.ts` desk actions, dock desk filters is dead code to delete | frontend/sessions |
| Frontend `SessionKind` | TS still declares `'desk'`; remove with the UI | frontend |
| Diff/blame/expanded caches | keyed `${repoId}/${path}` — collides once two worktrees of one repo exist; re-key to `${worktreeId}/${path}` | frontend |
| `activeRepoId` model | repo-centric switcher/commit-panel assumes one worktree per repo; becomes worktree-centric | sessions/frontend |
| `RepoDiff` grouping | `get_task_diff_summary` emits one entry per worktree; two worktrees of one repo produce two entries with the same `repo_id` — frontend must key by worktree | review/frontend |

### Decisions recorded for later phases (not implemented here)

- **Worktree directories**: ALL worktrees live at
  `<root>/worktrees/<session>/<project>@<branch-slug>` (slug: `/` → `-`).
  No un-suffixed form. → provisioning (worktrees phase).
- **Branch naming**: `<type>/<id>-<slug>`, generated as a *default from
  context* and user-editable at provision time (`BranchSpec.branch_name` is
  the override path). `derive_branch` becomes the default-suggester. →
  worktrees phase + provisioning UI.
- **Explorers get real branches** (`explorer/<name-slug>` off the default
  branch), created by the same code path as task branches —
  `provision_explorer_worktrees_impl` and the `--detach` flow are deleted.
  The `'(detached)'` sentinel dies, and with it: the `log_ref = HEAD` special
  case in `commits.rs`, the explorer→`working` diff-mode override in
  `get_task_diff_mcp`, and the frontend `diffMode: 'working'` explorer
  default. → worktrees phase.
- **Conversion** uses `git branch -m <explorer-branch> <task-branch>` instead
  of `switch -c` from detached HEAD. → sessions phase.
