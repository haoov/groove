# Groove — working in this repo

Tauri 2 desktop app: a task-driven workspace. One window holds a task, its repos,
their worktrees, the diff, the MR, and a Claude Code agent that can act on all of
it. React + TypeScript in `src/`, Rust in `src-tauri/`, SQLite for state, an MCP
server the agent talks to.

`README.md` is for users. This file is how to change the code.

## Verify

Run everything before claiming done. CI (`.github/workflows/ci.yml`) runs the same
list on every push and pull request; a red pipeline blocks the merge.

```sh
pnpm check                   # tsc, eslint, stylelint, dead-CSS check, vitest, vite build
cd src-tauri && cargo fmt --check
cd src-tauri && cargo clippy --lib --all-targets -- -D warnings
cd src-tauri && cargo test --lib   # all inline #[cfg(test)]
```

The pieces of `pnpm check`, when one is enough:

```sh
npx tsc --noEmit             # types
pnpm lint                    # eslint: defects + `groove/boundaries` (eslint/boundaries.js)
pnpm lint:css                # stylelint: no raw colour outside shared/styles
pnpm check:css               # every CSS class is referenced by a .ts/.tsx file
pnpm test                    # vitest — pure logic, node env
npx vite build               # catches what tsc alone does not
```

The Rust toolchain is pinned in `rust-toolchain.toml` and named again in the
workflow. `-D warnings` gates the build, so a floating channel would turn a new
clippy lint into a red pipeline on code nobody touched — bump both together.

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
  skills/          the agent's core prompt, its skills, and the plugin dirs for them
  home/            the Home snapshot          annotation_store/  line notes
  editor_host/     file reads and writes for the editor
src/
  app/             App (composition root) · chrome/ · providers/ (useIpc, useKeybindings)
  shared/          the frontend core — features import DOWN from here only
    ipc/           ipc.ts (THE type surface) · generated/ (ts-rs) · events.ts · ops.ts
    store/         zustand barrel · types.ts · session.ts (pure reducers) · slices/
    lib/           pure/ · actions/ · hosts/ · hooks/ + platform.ts, agentWindow.ts
    ui/  styles/
  home/ sessions/ workspace/ files/ git/ notes/ editor/ overview/ agent/ terminal/
  approvals/ notifications/ command/ setup/ actions/   — each owns its components + css
```

`shared/lib/` is grouped by what a module is allowed to touch:

| Bucket | Rule |
|---|---|
| `pure/` | No store, no IPC, no DOM, no xterm — importable from anywhere, tests included |
| `actions/` | Reads or writes the store, or calls `invoke` |
| `hosts/` | Stateful runtime objects with a lifecycle |
| `hooks/` | React hooks |

`platform.ts` and `agentWindow.ts` stay flat: `main.tsx` imports them by path.

A module used by exactly one feature lives in that feature, not here.

## Contracts

**Types are generated, one way.** Rust `#[ts(export)]` → `src/shared/ipc/generated/`
(63 types). Features import `shared/ipc/ipc` and **never** `generated/` — `ipc.ts`
re-exports the mechanical truth and rebuilds the deliberate narrowings (`DiffLine.type`,
`Annotation.status`, `UiConfig.theme`). Hand-write a type only where the frontend
narrows a field Rust cannot express, or no Rust struct exists (event payloads,
UI constants).

**Event names are generated too.** `core/events.rs`'s `events!` list is the only one;
`export_bindings_event_names` writes `generated/eventNames.ts`, and `shared/ipc/events.ts`
re-exports it. `ts-rs` emits types, not values, which is why a test writes this one. The
`backend_notice` payload is `BackendNotice` with a `NoticeKind`, generated like any type.

**Two hand-mirrored files.** Change one side, change the other in the same edit:
- approval ops — `approvals/ops.rs` ↔ `shared/ipc/ops.ts` (a Rust test guards both directions)
- MCP tool descriptions — `mcp_server/tools/definitions.rs` is the ONE place that tells
  the agent how to write a commit message, MR text, an annotation or a task body. Do not
  restate those rules in `agent/prompts.ts`, in a skill, or here; saying it in three
  places got it ignored in all three.

**Canned asks are skills, not strings.** The console's Actions drop-up renders
`list_agent_skills` and sends `/groove:<name>` through `sendSkill` — one menu, not a
pill each, so the bar does not grow with the skill list. Add an action by adding a
`SKILL.md` under `skills/core/` and a row in `CORE_SKILLS` — there is no prompt table
in the frontend.

**A skill's `description:` is written for the MODEL** — what it does, then `Use when …`
with the words a user types — because that is what Claude Code matches to invoke the
skill on its own, off the chat, with no button pressed. A test guards the phrase. The UI
shows `groove-hint` and never the description — writing the description short enough for a
tooltip is exactly what kills auto-invocation. A skill with no hint shows no line; there is
no fallback, because the fallback was the description.
A user's own live in `<config>/user-skills/skills/` and are written two ways: the
Settings manager (`src/actions/`) writes them directly, because that is the user typing,
and the agent writes them through `save_user_skill` — the one LOCAL write that is gated,
since a skill is an instruction it will later invoke unprompted. The bridge covers what
leaves the machine PLUS what authors the agent's future behaviour.

**A skill on disk is not a skill the agent has.** `--plugin-dir` is read at launch, so a
write marks `skillsStale` and the console offers a reload (`reloadAgent` — stop the PTY,
start it again, `--resume` keeps the conversation). The skills LIST deliberately does not
refresh until then: a pill for a skill the running agent cannot resolve answers with
"unknown command".

**Groove suggests, it never auto-sends.** A trigger renders a chip the user clicks
(`TaskOverview` offers `start-task` on a task with no repos, which is the one way in:
its step 3 attaches them, because provisioning a task without reading it first only
moves the guessing earlier).
Opening a session must never start an agent by itself — that spends tokens on a glance.
`ui.suggest_actions` hides every offer, and hides nothing else: the skill stays a button
and a slash command.

**Commands**: 108, registered in `lib.rs`'s `generate_handler!` — a test asserts every
`#[tauri::command]` has a row, since an unregistered one simply does not exist. `generate_handler!`
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
Frontend sigils and names come from `shared/lib/pure/forge.ts` only.

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
cwd carries no session context. The core prompt (`skills/prompt.md`, passed per launch
with `--append-system-prompt-file`) names the session, so a skill does not repeat it.
It is appended per launch and never baked into the conversation — edit it and every
session already open picks it up on its next start. Interpolate only STABLE identity;
repos, worktrees and MRs drift, and `get_active_task` is read when it is needed.

**Skills are gated by the launch flag.** `--plugin-dir` loads a plugin for ONE
session, and that is the only thing keeping Groove's skills out of the agents the user
starts from a terminal. Never install them into `~/.claude/skills` or the worktree root
instead — the root is the user's own directory and already holds their `CLAUDE.md`.

**Every outward write goes through the approvals bridge** — git commit/push/pull/rebase/
discard, MR create/update/close, and all task writes. Requests survive a crash and
re-surface at startup. `Commit & Push` posts the push only after the commit's
confirmation resolves approved.

**The agent commits its index; the UI commits everything.** `via_bridge` stamps
`index_only` on a `git.commit` from an agent, and `commit_impl` then refuses when
nothing is staged instead of falling back to `-a`. Those two branches are not
redundant: `-a` skips untracked files, so a new file beside a modified one was
committed without it, silently — and auto mode has no dialog to catch that. Staging
is also the only way an agent can commit four files out of ten.

**Git is subprocess-only**, funnelled through `core/git/run.rs` with `LC_ALL=C` forced
because call sites match English git output. There is no `git2`.

**PTY**: base64 both directions (`pty_output.b64`, `write_pty.dataB64`). `portable-pty`
never expands `~` — call `expand_tilde()` on any config path first. A desktop launch
carries no shell PATH, which is why `launch_env::widen_path()` runs before anything
spawns.

**Refresh contract**: `shared/lib/actions/refreshSession` = `flush_git_caches` → `invalidateDiff`
→ `refreshStatusFor` (+ `refreshHome` off-workspace). Driven by agent activity — only on
file-editing tools (throttled) and turn end, never every hook. There is no filesystem
watcher.

The flush is the sidebar button's, not the agent's: `refreshSession(id, false)` skips it,
because an agent edit moves the working tree and never a ref. Ref-moving ops — commit, push,
pull, rebase — flush in the backend where they run.

**A cached diff is not a fresh diff.** `invalidateDiff` bumps `diffNonce`, and every hunk,
blame and expanded-file fetch depends on it, so each refresh refetches and replaces in place.
The cached hunks stay on screen until the new ones land, which is what keeps "Loading diff…"
off an edit burst. Never gate one of those fetches on the cache being empty, and never infer
freshness from the summary's `added`/`deleted` counts: an edit that adds and removes one line
leaves them identical.

**Nothing polls the forge.** The CI chip moves on three things and no others: a
`git.push` or an `mr.*` op landing (`useIpc` bumps `mrNonce`), and the sidebar's refresh
button. `invalidateMrs` deliberately stays OUT of `refreshSession` — that runs on every
agent edit burst, and an API call per turn is what "no timers" was avoiding.

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
terminal` (tab/sidebar host); `git/ → editor/` (data → renderer); `settings/ → setup|
actions`; `agent/ → setup`; `overview/ → notes`. The list lives in
`eslint/boundaries.js` and `pnpm lint` fails on any other edge. The same rule keeps
`shared/ipc/generated` behind `ipc.ts`, `@tauri-apps/api/core` behind `invoke.ts`, and
the store internals behind the `shared/store` barrel.

**CSS**
- Tokens and themes only — Catppuccin, Latte default, config wins. Never a raw hex,
  named colour or numeric `rgb()` outside `shared/styles/tokens.css` and
  `shared/styles/themes/` — stylelint (`.stylelintrc.json`) fails on one.
- Every class selector must be referenced by a `.ts/.tsx` file. `pnpm check:css` lists
  the orphans; delete the rule, or add a runtime-built prefix to the script's allowlist.
- **No inset box-shadow left-border** for selected or active states. Use border +
  background.
- Watch cascade ORDER, not just specificity: `.x.variant` and `.y.variant` are both two
  classes, so the later one wins. The Home icon buttons and the create-task drop-up both
  needed a later block or a two-class selector for exactly this.
- The Home filter's highlight is a mirror `<div>` behind a transparent input. Both layers
  must keep **identical font, size, weight and spacing** — changing weight alone drifts
  the highlight off the caret.

**Comments**: say WHAT, and only where the code is not already explicit — not why. One
line is enough when one is needed, and none is often right. Keep a trap, a pin or a
guard (`do not change this until <condition>`) as one line. The why goes in the commit
body. The whole tree follows this rule; a multi-line why-block in a diff is a defect.

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
