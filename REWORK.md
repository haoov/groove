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
- **Branch naming**: `<type>/<slug>-<id>`, generated as a *default from
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

---

## Phase 2 — core/config: one source of truth (done)

- `Config` + sub-structs moved to `core/config`. One process-wide
  `RwLock<Option<Config>>`, initialised in `async_init` before anything reads
  it; the config dir is remembered there for every later save.
- API: `config::get()` / `require()` / `update(edit)` / `replace(cfg)` /
  `file_path()`. `update` is the single write path (UI prefs, setup) —
  persists **atomically** (temp + rename) and **0600** (S3 fixed).
- Deleted: `GLOBAL_CONFIG` static + `global_config()` (D12 fixed),
  `ensure_config(app, state)`, `save_ui`, the config half of
  `task_manager::State` (now only the active-session pointer).
- ~12 commands lost their `app: AppHandle` / `task_state` parameters —
  config is a plain read. IPC arg names unchanged; frontend untouched.
- Decisions: branch-type vocabulary for `<type>/<slug>-<id>` is hardcoded
  (conventional-commit list), not config; config reload stays restart-only.

### Notes for later phases

- `core/config::get()` clones; if a hot path ever shows up in timing, switch
  the static to `arc-swap`. Not warranted today.
- `check_environment` now reads the path via `config::file_path()` — the
  setup screen no longer needs an AppHandle for it.

---

## Phase 3a — core/git: one spawner, one resolver, a ref cache (done)

### Layout

- `core/git/run.rs` — `run()` / `output()` / `is_repository()`: the ONLY git
  spawner (spawn_blocking + LC_ALL=C + timing), enforced by the
  `no_git_spawn_outside_core_git` test.
- `core/git/url.rs` — `parse_git_url` (moved, with tests).
- `core/git/refs.rs` — `ref_exists` · `upstream_base` · `diff_base` ·
  `default_branch`: base_ref.rs absorbed, `resolve_default_branch` moved in,
  `mr_manager::detect_default_branch` DELETED (D1 dead — it stripped
  origin/HEAD and fell back to a literal "main", mis-targeting MRs on
  develop-default repos).
- `core/git/cache.rs` — `RefCache`: merge-base / ref-exists / default-branch
  answers cached with a **5s TTL**; `flush()` after every ref-moving op
  (commit, push, pull, rebase ×3, discard-all, fetch ×3, worktree
  add/close/switch). The fetch throttle (`due(key, window)`) lives here too;
  throttle stamps survive a flush.

### Also

- `ops.rs` impls rewritten on core::git (D2 dead): forced-English output,
  timing coverage, ~90 lines of spawn boilerplate gone.
- `editor_host::list_files` async on core::git (B17 fixed).
- **git2 dependency dropped** — both uses were "is this a repo?"; now
  `rev-parse --git-dir`.
- Fire-and-forget diff fetch flushes the cache on completion, so remote
  movement becomes visible on the next refresh.

### Effect

Diff refresh ~9 → ~4–5 spawns per worktree; file-diff tab 3 → 1; Home 4–7 →
2–3. Every git call now appears in `RUST_LOG=timing=debug`.

### Notes for later phases

- Staleness window: an agent committing in its own terminal can leave ref
  answers up to 5s stale; the debounced watcher refresh lands after that.
  If it ever bothers, watch `.git/HEAD` in the watcher and call
  `cache::flush()` — one line each side.
- `git_engine` is now operations-only (provision/pool/ops/status/watcher/
  diff/blame/parse/commits) — the 3b rename to `worktrees/` + `review/`
  split is mechanical.

---

## Phase 3b — worktrees feature: unified provisioning, new naming (done)

### Layout

`git_engine` is gone, split into:
- `worktrees/` — `naming.rs`, `pool.rs` (root/pool/session dirs, listing with a
  **60s cache**, clone, register), `provision.rs` (ONE path for all kinds),
  `teardown.rs`, `ops.rs`, `status.rs`, `watcher.rs`.
- `review/` — `diff.rs`, `blame.rs`, `commits.rs`, `parse.rs`, `types.rs`.

### Provisioning

- One core (`provision_one`): reuse the session's existing (repo, branch)
  worktree wherever its directory sits (old-layout dirs keep working), else
  freshen clone → ensure/track branch → `worktree add` at
  `<session>/<project>@<branch-slug>` → align → flush → upsert. Repos
  provision **concurrently** (P6).
- `provision_explorer_worktrees` command DELETED; explorers use the same path.
  `--detach` and the `'(detached)'` sentinel are gone, with their special
  cases (commits log_ref, MCP diff-mode override, frontend `working` default).
- Branch defaults (`worktrees/naming.rs`): tasks `<type>/<slug>-<id>` with the
  type inferred from whole words of the title; explorers `explorer/<name-slug>`;
  override via `BranchSpec.branch_name`. `task_manager::derive_branch` deleted.
- Directories: ALWAYS `<project>@<branch-slug>` for new worktrees.
- Conversion renames the branch (`git branch -m`) instead of `switch -c`, and
  relocates to the new dir shape.

### Notes for later phases

- **Provisioning UI** still sends `branch_name: null` everywhere; the wizard
  should show `naming::default_branch` as an editable prefill (needs a small
  `suggest_branch_name` command or the session title client-side) and the
  add-repo modal's task copy still names the old lowercase-id default. →
  frontend phase.
- The MCP explorer publish-guard (push/MR from explorers refused) is now
  **policy**, not mechanics — explorers have real branches. Kept as-is;
  lift it deliberately if ever wanted.
- Pre-rework explorer worktrees that are still detached on disk: conversion's
  `branch -m` fails on them (warning surfaces, worktree stays usable);
  commit-log for such a session errors. Discard/recreate is the path.
- `refresh_main_clone`'s serial fetch per repo is now concurrent across repos,
  still serial within one repo (correct: shared clone).

---

## Phase 3c — no watchers, no paused state, path-identity pool (done)

Backend only. The frontend was deliberately NOT touched: it still calls
removed/changed commands and is runtime-broken in the ways listed below.

### Paused state removed

- `sessions.state` column deleted (0006 edited in place), `SessionState` enum,
  `sessions::set_state` and the migration's paused derivation gone.
- `pause_task` now only clears the active pointer and emits `task_paused` —
  "pausing" is purely a UI close. Rename the command in the sessions phase.

### Watchers removed — explicit refresh is the model

- `worktrees/watcher.rs`, `worktrees::State`, the `watch_task_worktrees`
  command, the `file_changed` event and the **notify dependency** are gone.
- New command `flush_git_caches` — the "make the next refresh exact" half of
  the explicit-refresh contract.
- Auto-refresh is now driven by **agent activity**, not the filesystem: the
  hooks (`agent_activity` events) fire per agent action; refresh throttled
  (~2s) while `working`, exactly once on the working→idle transition.

### Pool: the path IS the identity

- `MainRepo` = `{ local_path, slug }` — **no url field**. Listing is a pure
  directory walk, zero git subprocesses, no cache needed (cache deleted).
- `slug_parts(slug)` derives (host, group, project); review-queue matching
  uses slugs directly instead of parsing origin URLs.
- `register_repo(slug, local_path)` — signature changed. The single
  `remote get-url origin` check happens HERE, at attach, as validation that
  forge features can work. `register_repo_impl` same. Review attach builds the
  slug from `url_host(web_url) + project_full` (new `core::git::url_host`).

### Frontend work owed (runtime-broken until done) — frontend phase

| Break | Fix |
|---|---|
| `useIpc` invokes `watch_task_worktrees` on every workspace_ready → error toast | delete `startWatchers` + the call |
| `file_changed` listener + 350ms debounce block | delete; never fires |
| No auto-refresh wiring | add `refreshSession(id)`: `flush_git_caches` → invalidateDiff → refreshStatusFor (+ refreshHome off-workspace); call it from `agent_activity` (throttled while working, once on idle) and from the sidebar refresh button |
| `register_repo` invoked with `{localPath, remoteUrl}` (repoPicker) | pass `{slug, localPath}`; `MainRepo.url` no longer exists (type + tooltip) |

## Phase 3d — core/pty: mechanics extracted, trace deleted, bash imposed (done)

Backend only. Frontend untouched; owed changes below.

### New module `core/pty`

- `Ptys` registry (managed state, replaces `agent_manager::State`):
  `spawn(app, PtySpec) -> session_id`, `write`, `resize`, `kill`.
- `PtySpec { task_id, kind, cwd, program, args, env, on_exit }`. No uniqueness
  constraint — a session opens any number of PTYs (multiple terminals already
  work end to end; nothing changed there).
- Reader loop (base64 `pty_output`, EOF reap, `pty_exit`) and
  `describe_terminal` (TERM/COLORTERM) live here. Domain-free: the
  agent-death → `agent_hooks::forget` link is now an `on_exit` callback passed
  by `agent_manager`.
- Generic IPC commands moved here, names/shapes frozen: `stop_agent_session`,
  `write_pty`, `resize_pty`.
- Kill is `libc::kill(pid, SIGTERM)` (new direct dep `libc`), not a spawned
  `kill` subprocess. Entry dropped + immediate `pty_exit` as before; reader EOF
  still emits a second `pty_exit` (frontend handlers are idempotent).

### agent_manager slimmed to policy

- Keeps: session UUID derivation, legacy id fallback, `resolve_claude_bin`,
  `resolve_root_cwd`, `hook_settings`, `claude_env` (MCP timeout vars, set on
  every PTY as before), `start_agent_session`, `start_terminal_session`,
  `start_login_pty` (now sync).
- **Shell imposed**: terminals and the auth PTY run `TERMINAL_SHELL =
  "/bin/bash"`, never `$SHELL`. One redraw behavior against xterm.js. `.zshrc`
  aliases are gone from in-app terminals by decision; PATH still comes from
  `launch_env::widen_path`.
- `resolve_confirmation` command moved to `confirmation_bridge` (handler path
  change only).
- `expand_tilde` moved to new `core/fs.rs` (users: setup, pool, agent_manager).

### pty_trace deleted

- `pty_trace.rs`, its `init` call, and the `trace_pty` / `pty_trace_on`
  commands are gone. The doubled-character bug it existed to attribute is
  fixed; bash removes the zsh variable entirely.

### Frontend work owed — frontend phase

| Break | Fix |
|---|---|
| `src/lib/ptyTrace.ts` invokes removed `pty_trace_on`/`trace_pty` (degrades silently) | delete the file + `tracePty` import/calls in `terminalHost.ts` |
| `write_pty` sends `data: Array.from(bytes)` (P9, number array on the hottest path) | send base64 both ways; change `write_pty`/`data` contract together with the backend in that phase |

## Phase 3e — core/http: one shared client (done)

Closes P2. Backend only; no IPC or frontend impact.

- New `core/http.rs`: `client()` returns the process-wide `reqwest::Client`
  (`OnceLock`). One connection pool — Notion calls reuse a live TLS connection
  instead of building a client + handshake per request.
- **Timeouts introduced (behavior change, by decision)**: connect 10s, total
  30s. Before this a hung call blocked its command forever.
- `notion.rs`: `notion_client` deleted; auth/version headers set per request;
  the triplicated get/post/patch bodies collapsed into one `notion_call`.
  Names, signatures, and error shapes of the three verbs unchanged — all 20
  call sites untouched.
- Guard test `no_http_client_outside_core_http`, same style as the git-spawn
  and raw-SQL guards.

## Phase 3f — core/events: relocation only (done)

- `src/events.rs` → `src/core/events.rs`, references now `crate::core::events`.
- Nothing else changed: same names, same `set_app`/`notice` globals, same
  hand-mirrored contract with `src/lib/events.ts`.

## Phase 4 — notion feature module (done)

Backend only. All Notion API knowledge now lives in `src/notion/`; `api.rs`
holds the verbs as `pub(super)` (compiler-enforced boundary) plus a guard test
(`no_notion_api_outside_notion_api`). task_manager drops to session lifecycle,
time ledger, repos, conversion, setup.

### Layout

`api` (verbs + paginate_get/paginate_post), `markdown` (blocks↔md, pure),
`schema`, `detect` (moved as-is), `page` (page_to_task + canonical property
shapes), `properties`, `tasks` (queue query, sync, set_status, trash, sprint
cache), `create` (NewTask, create_page, filing), `body` (read/replace/template),
`hours` (Notion half; ledger stays in task_manager), `users` (find by email).

### Dropped

- **`notion.status` op + MCP tool `update_notion_status` + impl** — redundant
  with `notion.property` (status is a property type), and the dropped path never
  refreshed the local mirror (staleness bug dies with it).
- **`list_notion_users`** → `find_notion_user(token, email)` returns the one
  matching person. No fallback by decision; requires the integration's
  user-email capability.
- `inject_notion_secrets` and the `token` field in op payloads — handlers read
  config at execution time; secrecy (nothing persisted) unchanged.
- Title name-guessing in page_to_task (exact by-type scan stays).

### Fixed / improved

- `sync_task` filtered on a property literally named "unique_id"; now resolves
  the unique_id property NAME from the schema (suspected 400 on every sync).
- Sprint-status property resolved by TYPE in the sprint DB's schema — last of
  the N2 hardcoded vocabulary ("Sprint status") gone. "Current" match stays.
- Current-sprint ids cached 5 min: task listing pays 1 Notion call amortized,
  not 3.
- `create_task` extra properties: one batched PATCH, per-property fallback only
  on failure (keeps warning isolation).
- Body reader fetches table rows concurrently.
- `list_tasks` mirrors pages in one transaction.
- Three hand-rolled cursor loops → `api::paginate_get/paginate_post`.

### Frontend work owed — frontend phase

| Break | Fix |
|---|---|
| `FirstRun.tsx` invokes removed `list_notion_users` | email input → `find_notion_user({token, email})` → `{id, name, email}` |
| `notion.status` confirmation renderer (ops.ts + modal) is dead | delete; `notion.property` covers status |

All other IPC names/shapes frozen.
