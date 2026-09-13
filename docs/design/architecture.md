# Architecture

A Cargo workspace in five layers. The simplest code is at the bottom, called by the
layer above it. Nothing calls up.

## Layers

| Layer | Crates | Role |
|---|---|---|
| ui | `groove` (bin) · `ui` · `mcp-server` | present, translate input |
| controllers | `task` · `session` · `workspace` · `agent` · `config` | one per service; one function per action, services in order |
| services | `task` · `session` · `workspace` · `agent` · `config` | one per capability |
| modules | `provider` · `worktree` · `git` · `diff` · `annotations` · `editor` · `text` · `terminal` · `agent-launch` · `forge` · `tools` · `hooks` · `activity` · `approvals` · `timeline` · `skills` · `config` · `watch` | one concern each |
| base | `db` · `http` · `exec` · `gfx` | one way out of the process each |
| shared | `types` | the vocabulary: data and pure rules, used from modules up; depends on nothing; the base never touches it |

## Who does what

| Layer | Does | Never |
|---|---|---|
| **base** | one way out of the process: disk state, network, a process, the GPU | holds application logic; knows a domain type |
| **shared** | `types`: the data every layer talks in, and pure rules over it | IO, async, anything with a side effect |
| **modules** | one concern, on the base: parse a diff, provision a worktree, talk to GitLab; IO exposed only as `async fn` | calls a service; knows a user action; offers a blocking path to IO |
| **services** | one capability: its slice of `AppState` and the operations on it, assembled from modules | calls another service; orders work across capabilities |
| **controllers** | one module per service, one function per user action: which services, in what order, what to record, what to undo on failure | touches a module directly; holds state |
| **ui** | draws `AppState` as a `Frame`; turns input into a controller call | calls a service; decides an order |

Rules that follow:

- A service depends on modules and `types`, never on a service. A module depends on
  the base and `types`, never on a module above it. No service crate
  depends on another, so a cycle cannot compile.
- A controller function is the only place two capabilities meet, and the only place
  that knows what to undo when a later step fails.

  ```
  controllers::task::open(state, services, id)
      services.session.create(id)
      services.task.set_status(id, InProgress)
      services.workspace.provision(id)
      services.agent.start(id)
  ```

- Controllers mirror the services: `task`, `session`, `workspace`, `agent`, `config`.
  A function is the entry point for one action whether it calls one service or four;
  `session.select_worktree` calls one, `session.open` calls four.
- A read is a controller function too: it returns data and mutates nothing.
  `task.get`, `workspace.get_diff`, `session.get_active`. One enum, one `dispatch`;
  the MCP tools that read map to these.
- The command id is `controller.function`: `task.open`, `session.add_repo`,
  `agent.approve`, `workspace.push`. That one string is the palette entry, the MCP
  tool name, the keybinding target and the timeline label. The palette shows a label
  and a group — *Workspace › Git › Push*; the id is for code and tools.
- The controllers **are** the API. A function nobody can reach, and a palette entry
  or tool that is not a function, are both defects.
- `AppState` is the sum of the services' slices, owned on the main thread. A controller
  receives it whole and hands each service its slice.

## What each service assembles

| Service | Modules | Base |
|---|---|---|
| task | provider (notion, github issues) | http |
| | timeline | db |
| session | worktree (clone pool, provisioning, naming) → git | exec, db |
| | agent-launch (flags, core prompt) → terminal | exec |
| | terminal (emulation) | exec |
| | approvals | db |
| workspace | git (conventions, parsers, actions) | exec |
| | diff (modes, hunks, blame, expansion) → git, text | exec |
| | watch (selected worktree) | — |
| | annotations → forge | db, http |
| | editor (file ops) → text | — |
| | text (rope, tree-sitter, transactions) | — |
| | terminal (emulation) | exec |
| | forge (gitlab, github PRs) | http, exec (tokens from gh, glab) |
| agent | tools (definitions, tiers) → approvals | db |
| | hooks (receiver) → activity → timeline | db |
| | skills (core, user, on disk) | — |
| | approvals | db |
| config | config (file) | — |
| | environment check | exec |

## Crates

One directory per layer under `crates/`.

| Layer | Crate | Holds |
|---|---|---|
| base | `db` | pool, migrations, `test_pool` |
| | `http` | one client: auth header, timeout, retry, redaction |
| | `exec` | a process: `run` to completion with capture, timeout, kill on drop, redaction; `pty` with a terminal, streaming, resize |
| | `gfx` | wgpu device, glyph atlas, cell grid, box quads, theme |
| shared | `types` | session, task, worktree, repo, mr, diff, annotation, activity, delivery, timeline event, config, error, ids, `Generation`; pure rules such as `names_session` and the word-diff pair rule |
| modules | `provider` | notion, github issues, the registry |
| | `worktree` | clone pool, provisioning, naming, teardown |
| | `git` | the git client: environment conventions, parsers for status, diff, blame, log, the actions |
| | `diff` | modes, hunks, commits, blame, expansion |
| | `annotations` | store, post to MR |
| | `editor` | file ops, open-file state |
| | `text` | rope, tree-sitter, transactions, semantic hook |
| | `terminal` | emulation: `alacritty_terminal` grid over `exec::pty`; terminals per worktree |
| | `agent-launch` | flags, session UUID, core prompt, the agent's terminal |
| | `forge` | gitlab, github PRs, review queue |
| | `tools` | MCP tool definitions, arguments, the tier table |
| | `hooks` | the loopback receiver for the agent's hooks |
| | `activity` | agent status per session |
| | `approvals` | the bridge: queue, record, resolve |
| | `timeline` | one log per session |
| | `skills` | core and user skills, plugin dirs |
| | `config` | the config file |
| | `watch` | filesystem watcher on the selected worktree, debounced |
| services | `task` `session` `workspace` `agent` `config` | one per capability; its slice of `AppState`, its operations, its `Event` variants and `apply` |
| controllers | `controllers` | one module per service, one function per action; the `Command` enum and `dispatch`; `Spawner`, `Continuation`, cancellation |
| ui | `ui` | `frame.rs`, `layout.rs`, `input.rs`, `widget/`, `view/<capability>/` |
| | `mcp-server` | axum; each tool calls one controller |
| | `groove` | the binary: winit, event loop, wiring |

`ui` never calls `gfx` directly; it emits a `Frame`.

## State and threads

- winit's event loop on the main thread owns `AppState`, the sum of the services'
  slices.
- Every IO call runs on a tokio pool. Its result returns as a continuation through
  `EventLoopProxy`; the outside world returns as an `Event`.
- Mutations happen only in `dispatch`, a continuation or `apply`. One writer, one
  thread.
- Render reads `&AppState`. No second copy, no cache nonce, no sync.
- No `Arc<Mutex<AppState>>`.
- Periodic work — the forge poll, the fetch throttle — is spawned by the services on
  the same pool and reports through the same channel. Window focus is an input to it.

## Commands, continuations, events

Commands go down as data. Results come back as continuations. Events are the outside
world. One thread applies all three to `AppState`.

**Command.** An enum grouped by controller, one variant per function:
`Command::Task(task::Command::Open { id })` is `task.open`. Produced by ui input, the palette, a keybinding or an MCP tool; never
a direct call. `dispatch(cmd, &mut AppState, &Services, &Spawner)` is one match that
calls the controller function.

**Controller function.** Two parts. The sync part mutates `AppState` now — mark the
session *setting up*, select the worktree. The async part is spawned through the
`Spawner` with the session's `CancellationToken`, and carries its continuation.

**Continuation.** `Box<dyn FnOnce(&mut AppState) + Send>`, delivered through winit's
`EventLoopProxy`, run on the main thread. It writes the result into the owning
service's slice, records to the timeline when it should, and sets the dirty flag.

```rust
spawn(cx, token, async move { load_diff(wt).await })
    .then(|state, diff| { state.workspace.diff_loaded(wt, diff); });
```

**Generation.** Every job is tagged `(session, kind, generation)`. The continuation
compares against the current generation in `AppState` and drops a stale result. One
rule, one place.

**Event.** External inputs with no originating command, applied by `apply(event,
&mut AppState)` — one match, one line per variant, delegating to the owning service.

| Event | From |
|---|---|
| `Agent::Hook { session, kind, tool }` | the hook receiver |
| `Agent::Exited { session, status }` | the agent's PTY |
| `Terminal::Damaged { id }` | the reader thread, coalesced per frame |
| `Approvals::Decided { id, approved }` | the user, on the rail |
| `Forge::Polled { mr, details, ci, threads }` | the poll |
| `Workspace::FilesChanged { worktree, paths }` | the filesystem watcher, debounced |
| `Window::Focus(bool)` | winit |
| `Config::Changed` | the config file |

**Firehoses never enter the loop.** The terminal module's reader thread parses bytes
into the grid and sets a dirty flag; one `Terminal::Damaged` per frame at most. Hook
bursts on file-editing tools are coalesced before they become a refresh.

**Keystrokes never touch a device on the main thread.** The terminal module's writer
thread owns the PTY's input side; `agent.send` and `agent.resize` queue on its channel
and return. The channel keeps the order typed.

**Redraw.** `ControlFlow::Wait`. A dirty flag set by any continuation or `apply`;
one `request_redraw`; one `view(&AppState) → Frame` per batch.

**Timeline.** Written explicitly, by continuations and by `apply`, to one table. The
main thread's arrival order is the true order.

**Tests.** A `SyncSpawner` runs the future and its continuation at once, so
`dispatch` returns with the work complete and the test asserts `AppState`. Tracing
tags every job with its command id.

**The one rule the compiler cannot enforce:** no IO in `dispatch`, `apply` or a
continuation. Lint and review.

## `Frame`

```
view(&AppState, &mut Frame)  →  Frame { quads, text_runs, clips, hit_regions }
                                          ↓
                                 gfx renders Frame
```

`Frame` is a display list rebuilt every frame from `&AppState`. Widgets emit it; the
renderer draws it. Neither crate sees the other's types.

## Chrome

Owned, minimal. About eight primitives on `gfx`: row/column, list, text,
button, input, modal, splitter, tab bar. Plus focus, hit testing, scroll. No general
toolkit. `widget/` and `view/` stay separate.

No layout engine. Nested splits and fixed-height lists. Text flow belongs to the
`text` module.

## Rendering

`gfx` draws a cell grid for the terminal and the editor, and rects and text
runs for the chrome.

- **One glyph per cell.** Exact column alignment; single-glyph shaping caches to a
  small set. No ligatures. Wide characters take two cells.
- **Box and block glyphs are quads.** Each glyph decodes into per-side arm weights
  and draws as rectangles that tile exactly. Light, heavy, dashed, corners, tees,
  crosses, stubs, full/half/eighth/shade blocks. Double-line and rounded glyphs stay
  on the font.
- **Never read the frame back.** The window presents.
- Rounded rects and borders: one SDF fragment shader. Clipping: a scissor per batch.
- Terminal module: `alacritty_terminal` for VTE, grid and scrollback. Its master fd is
  non-blocking; the reader sleeps on `WouldBlock`. Resize goes to the PTY through
  `OnResize::on_resize`, not only to the `Term`.

Stack: wgpu 30 · glyphon 0.12 (cosmic-text 0.19) · alacritty_terminal 0.26 · ropey ·
tree-sitter.

## Tests

| Layer | How |
|---|---|
| base | each crate standalone: `test_pool` in-memory; a mock server for `http`, redaction asserted on the wire; a scripted binary for `exec`; a scripted binary for `exec::run`, a spawned shell for `exec::pty`; golden images from `Frame` fixtures for `gfx` |
| modules | each against the base: `forge` and `provider` on a fake `http`; `terminal` asserts the grid from a byte stream; `text` pure; `git` on temp repos with repo-local identity and `commit.gpgsign=false`; `tools` — every tool definition maps to one controller |
| services | each in isolation, on fakes of its modules |
| controllers | drive the API headlessly, assert `AppState` |
| ui | assert the `Frame` against a fixture `AppState`; no GPU |

Requirements:

- Golden images run on software Vulkan (`mesa-vulkan-drivers`, lavapipe) in CI.
- The fonts are vendored in the repo: IBM Plex Sans and IBM Plex Mono, OFL.

- [ ] Prove lavapipe on `ubuntu-latest`.
- [ ] A chrome probe: one real pane drawn with owned primitives.

## Migration

The base and `types` first, then the spine, then one vertical slice at a time. A
module lands with its first consumer and only as wide as that consumer needs.

1. Base and shared: `db`, `http`, `exec`, `gfx` from the probe; `types` from the
   scattered models.
2. The spine: the five services as empty slices, the controllers crate with
   `AppState`, `Command`, `dispatch`, `Event`, `apply` and the `Spawner`; `ui` with
   the rail and an empty session surface; the `groove` binary with the loop.
3. Slices, each ending in the window:
   1. a session with its agent — `terminal`, `agent-launch`, the agent pane
   2. sessions and worktrees — `worktree`, the provisioning part of `git`, the rail
   3. the workspace diff — the rest of `git`, `diff`, `text`, the files and diff tabs
   4. the board and tasks — `provider`, the three columns, the filter
   5. MR and forge — `forge`, the header's MR and CI, the review sheet
   6. asks — `tools`, `mcp-server`, `hooks`, `activity`, `approvals`, `timeline`
   7. settings — `config`, `skills`, `watch`

The layer test in `controllers` reads every crate's manifest and refuses a
dependency that points up or across the services.

## Removed at parity

Tauri, ts-rs and `generated/`, the event-name test, `eslint/boundaries.js`, stylelint,
the CSS token system, vitest, vite, `scripts/check-css-classes.mjs`.
