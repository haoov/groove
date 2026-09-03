# Groove — working in this repo

Tauri 2 desktop app: a task-driven workspace. One window holds a task, its repos,
their worktrees, the diff, the MR, and a Claude Code agent that can act on all of
it. React + TypeScript in `src/`, Rust in `src-tauri/`, SQLite for state, an MCP
server the agent talks to.

`README.md` is for users. This file is how to change the code.

## Verify

Run all four before claiming done. There is no CI.

```sh
pnpm test                    # vitest — 12 files, pure logic, node env
npx tsc --noEmit             # types
pnpm lint                    # eslint (src only; formatting is not linted)
cd src-tauri && cargo test --lib   # ~193 tests, all inline #[cfg(test)]
npx vite build               # catches what tsc alone does not
```

`pnpm gen:types` after ANY change to a `#[ts(export)]` Rust type — it runs
`cargo test --lib export_bindings` then rebuilds the barrel via
`scripts/gen-index.mjs`. A renamed or deleted Rust type leaves a stale `.ts`
behind; delete it by hand.

**Do not run `pnpm tauri dev` or `pnpm tauri build`** unless asked. The owner keeps
`pnpm dev` running with HMR, so UI work is already live; `tauri dev` migrates the
real database and a release build takes a minute for nothing.

## Layout

```
src-tauri/src/
  core/            promoted shared code — db, git, config, pty, forge(api), events, timing
  provider/        task sources behind TaskProvider: notion/, github/, + write.rs (generic)
  task_manager/    sessions, setup, repos, time, explorer→task conversion
  worktrees/       provisioning, naming, teardown, the clone pool
  forge/           MR/PR operations (glab, gh)
  review/          review sessions and their diffs
  approvals/       the confirmation bridge — every outward write passes here
  mcp_server/      the tools the agent calls
  agent_manager/   agent + terminal PTYs      agent_hooks/  activity callbacks
  home/            the Home snapshot          annotation_store/  line notes
  editor_host/     file reads and writes for the editor
src/
  app/             App (composition root) · chrome/ · providers/ (useIpc, useKeybindings)
  shared/          the frontend core — features import DOWN from here only
    ipc/           ipc.ts (THE type surface) · generated/ (ts-rs) · events.ts · ops.ts
    store/         zustand barrel · types.ts · session.ts (pure reducers) · slices/
    lib/  ui/  styles/
  home/ sessions/ workspace/ files/ git/ notes/ editor/ overview/ agent/ terminal/
  approvals/ notifications/ command/ setup/     — each owns its components + css
```

## Contracts

**Types are generated, one way.** Rust `#[ts(export)]` → `src/shared/ipc/generated/`
(51 types). Features import `shared/ipc/ipc` and **never** `generated/` — `ipc.ts`
re-exports the mechanical truth and rebuilds the deliberate narrowings (`DiffLine.type`,
`Annotation.status`, `UiConfig.theme`). Hand-write a type only where the frontend
narrows a field Rust cannot express, or no Rust struct exists (forge JSON, event
payloads, UI constants).

**Three hand-mirrored files.** Change one side, change the other in the same edit:
- events — `core/events.rs` ↔ `shared/ipc/events.ts` (18 names)
- approval ops — `approvals/ops.rs` ↔ `shared/ipc/ops.ts` (a Rust test guards both directions)
- MCP tool descriptions — `mcp_server/tools/definitions.rs` is the ONE place that tells
  the agent how to write a commit message, MR text, an annotation or a task body. Do not
  restate those rules in `agent/prompts.ts` or here; saying it in three places got it
  ignored in all three.

**Commands**: 102, registered in `lib.rs`'s `generate_handler!`. `generate_handler!`
resolves `__cmd__*` symbols at the path you name, so moving a command between modules
is fine as long as a `pub use` keeps the registered path resolving.

## Invariants

Break these and something fails quietly.

**Providers.** `provider/mod.rs`'s `REGISTRY` is the single enumeration point, sized by
`ProviderId::ALL.len()` so a new variant refuses to compile until its row exists. Its
doc header lists every edit site a new provider needs. Never branch on a provider
outside `provider/`; go through `resolve()`/`get()`.

**`provider` is not `forge`.** A provider is where the TASK came from (notion, github);
a forge is where the CODE is hosted (github, gitlab). An MR has no provider. Both can
read "github", which is exactly why one key for the two answers the wrong question.
Frontend sigils and names come from `shared/lib/forge.ts` only.

**`short_id` is identity.** It is the session's primary key and it lands in branch names
(`fix/parser-plat-42`) and therefore in worktree paths. Minted once, never recomputed —
it names directories that already exist. Never a raw `external_id`.

**A task's provider comes from its row's column**, never from the id's shape. Any uuid
used to be treated as Notion, which silently mis-routed a uuid-keyed provider.

**Worktree-centric.** `activeWorktreeId` is the git-ops target; `activeRepoId` derives
from it. Diff/blame/expansion caches key on `${worktreeId}/${path}`. Worktree dirs are
`<project>/<branch>` and the branch keeps its slashes as real directories — so the last
path segment is the BRANCH leaf, not the repo. Use the payload's `repo` for display, and
`create_dir_all` the parent before `git worktree move`.

**Agents always start at the worktree ROOT**, never inside a repo or task directory, so
every prompt must name its session explicitly (`shared/lib/agentSend`, `agent/prompts.ts`).

**Every outward write goes through the approvals bridge** — git commit/push/pull/rebase/
discard, MR create/update/close, and all task writes. Requests survive a crash and
re-surface at startup. `Commit & Push` posts the push only after the commit's
confirmation resolves approved.

**Git is subprocess-only**, funnelled through `core/git/run.rs` with `LC_ALL=C` forced
because call sites match English git output. There is no `git2`.

**PTY**: base64 both directions (`pty_output.b64`, `write_pty.dataB64`). `portable-pty`
never expands `~` — call `expand_tilde()` on any config path first. A desktop launch
carries no shell PATH, which is why `launch_env::widen_path()` runs before anything
spawns.

**Refresh contract**: `shared/lib/refreshSession` = `flush_git_caches` → `invalidateDiff`
→ `refreshStatusFor` (+ `refreshHome` off-workspace). Driven by agent activity — only on
file-editing tools (throttled) and turn end, never every hook. There is no filesystem
watcher.

**Store**: one zustand store from full-state slices; every consumer imports the barrel.
The `buildView` WeakMap and bound-action caches are perf invariants — a session object's
identity changes only on real change.

**Migrations are append-only.** Never edit a shipped file (sqlx checksums it); add
`00NN_*.sql`. `sessions` parents six `ON UPDATE CASCADE` children, so rebuilding it needs
`PRAGMA foreign_keys=OFF`, which cannot run inside sqlx's per-migration transaction —
prefer ALTER there.

## Frontend conventions

**Dependency direction**: features import from `shared/` and themselves. Declared
exceptions: `app/ → *` (composition root); `workspace/ → files|git|notes|editor|overview|
terminal` (tab/sidebar host); `git/ → editor/` (data → renderer); a host may import a
leaf feature.

**CSS**
- Tokens and themes only — Catppuccin, Latte default, config wins. Never a raw hex.
- **No inset box-shadow left-border** for selected or active states. Use border +
  background.
- Watch cascade ORDER, not just specificity: `.x.variant` and `.y.variant` are both two
  classes, so the later one wins. The Home icon buttons and the create-task drop-up both
  needed a later block or a two-class selector for exactly this.
- The Home filter's highlight is a mirror `<div>` behind a transparent input. Both layers
  must keep **identical font, size, weight and spacing** — changing weight alone drifts
  the highlight off the caret.

**Comments**: say WHAT, and only where the code is not already explicit — not why. One
line is enough when one is needed, and none is often right. Existing long explanatory
blocks are legacy; do not copy that density into new code.

## Gotchas

- One Groove at a time: the MCP port (`127.0.0.1:27413`, tools reach the agent as
  `mcp__groove__*`) and the SQLite database are both single-owner.
- Config: `~/.config/com.haoov.groove/workbench.config.json`. State:
  `~/.local/share/com.haoov.groove/app.db`. The bundle identifier decides both — do not
  change it.
- `test_pool()` (`core/db/mod.rs`) is in-memory with `max_connections(1)` — a second
  connection would see a different empty database.
- `provider/notion/` still hosts Notion-specific code only; the provider-generic body and
  property writes live in `provider/write.rs`.
