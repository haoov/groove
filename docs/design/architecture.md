# Architecture

A Cargo workspace in six layers. The simplest code is at the bottom, called by the
layer above it. Nothing calls up. This file holds the rules; the code holds the how.

## Layers

| Layer | Crates | Role |
|---|---|---|
| ui | `groove` (bin) · `ui` · `mcp-server` | present, translate input |
| controllers | `controllers` | one module per service; one function per action |
| services | `task` · `session` · `workspace` · `agent` · `config` | one per capability |
| modules | `provider` · `sessions` · `worktree` · `git` · `diff` · `grep` · `annotations` · `editor` · `text` · `terminal` · `agent-launch` · `token` · `forge` · `tools` · `hooks` · `activity` · `approvals` · `timeline` · `skills` · `config` · `watch` | one concern each |
| base | `db` · `http` · `exec` · `gfx` | one way out of the process each |
| shared | `types` | the vocabulary: data and pure rules, used from modules up; depends on nothing; the base never touches it |

## Who does what

| Layer | Does | Never |
|---|---|---|
| **base** | one way out of the process: disk state, network, a process, the GPU | holds application logic; knows a domain type |
| **shared** | `types`: the data every layer talks in, and pure rules over it | IO, async, anything with a side effect |
| **modules** | one concern, on the base; IO exposed only as `async fn` | calls a service; knows a user action; offers a blocking path to IO |
| **services** | one capability: its slice of `AppState` and the operations on it | calls another service; orders work across capabilities |
| **controllers** | one function per user action: which services, in what order, what to undo on failure | touches a module directly; holds state |
| **ui** | draws `AppState` as a `Frame`; turns input into a controller call | calls a service; decides an order |

Rules that follow:

- A service depends on modules and `types`, never on a service. A module depends on the
  base and `types`, never on a module above it.
- A controller function is the only place two capabilities meet, and the only place that
  knows what to undo when a later step fails.
- Controllers mirror the services: `task`, `session`, `workspace`, `agent`, `config`. One
  function is one action, whether it calls one service or four.
- A read is a controller function too, and mutates nothing. One enum, one `dispatch`.
- The command id is `capability.action`: `task.open`, `workspace.push`. That one string is
  the palette entry, the MCP tool name, the keybinding target and the timeline label. The
  namespace stays flat however the code is split.
- The controllers **are** the API. A function nobody can reach, and a palette entry or
  tool that is not a function, are both defects.
- `AppState` is the sum of the services' slices, owned on the main thread. A controller
  receives it whole and hands each service its slice.

## Splitting what grows

The capabilities are five and do not move. Growth is answered inside them.

- **One service crate per capability, never two.** When it grows, split it into files by
  the features [capabilities.md](capabilities.md) already names — for workspace: diff,
  annotations, editor, terminal, git, forge.
- **A controller splits the same way**, into a directory beside its file. The `Command`
  enum and `dispatch` stay in the file, so the action list is readable in one place.
- **A view file draws one region.** Over the ceiling it becomes a directory named after
  the region, one file per part. No bucket names: a directory is a region, never
  `components`.
- **Ui state sits with its view.** `Ui` keeps only what crosses views: focus, the splits,
  the palette, the colour cache.
- **One module convention: `name.rs` beside `name/`.** No `mod.rs`.
- **Ceilings, enforced by the layer test:** 300 lines a file, 50 lines a function; a test
  file may stand to 500 and a test to 100, since a test is one scenario. Examples are
  exempt.

## What each service assembles

| Service | Modules | Base |
|---|---|---|
| task | provider (github issues, notion) · timeline | http, exec, db |
| session | worktree → git · agent-launch → terminal · terminal · approvals | exec, db |
| workspace | git · diff → git, text · grep · watch · annotations · editor → text · text · forge | exec, db, http |
| agent | tools → approvals · hooks → activity → timeline · skills · approvals | db |
| shell | terminal | exec |
| config | config · environment check | exec |

## Crates

One directory per layer under `crates/`. What each holds:

| Crate | Holds |
|---|---|
| `db` | pool, migrations, in-memory database for tests |
| `http` | one client: auth header, timeout, retry, redaction; a GraphQL endpoint |
| `exec` | a process: run to completion with capture, timeout, kill on drop, redaction; a pty with streaming and resize |
| `gfx` | wgpu device, glyph atlas, cell grid, box quads, theme |
| `types` | the vocabulary and the pure rules over it |
| `provider` | github issues, notion, and the enum that registers them |
| `sessions` | the session rows |
| `worktree` | clone pool, provisioning, naming, teardown |
| `git` | environment conventions, the parsers, the actions |
| `diff` | alignment, the whole change as one surface, the open file, word diff |
| `grep` | text across a worktree, walked in parallel, reported in batches |
| `annotations` | the notes a session leaves, as its own store |
| `editor` | the path operations under one worktree, and the clipboard |
| `text` | rope, tree-sitter, transactions, semantic hook |
| `terminal` | emulation over `exec::pty`: the agent's, and each session's own |
| `agent-launch` | flags, session uuid, core prompt, the agent's terminal |
| `token` | a host's bearer token, asked of `gh` or `glab`, held for the run |
| `forge` | gitlab, github PRs, review queue |
| `tools` | MCP tool definitions, arguments, the tier table |
| `hooks` | the loopback receiver for the agent's hooks |
| `activity` | agent status per session |
| `approvals` | queue, record, resolve |
| `timeline` | one log per session |
| `skills` | core and user skills, plugin dirs |
| `config` | the files the app keeps beside the database |
| `watch` | filesystem watcher on the selected worktree, debounced |
| `ui` | layout, input, styles, tokens, widgets, the views |
| `mcp-server` | axum; each tool calls one controller |
| `groove` | the binary: winit, the event loop, the wiring |

`ui` never calls `gfx` directly; it emits a `Frame`.

## State and threads

- winit's event loop on the main thread owns `AppState`.
- Every IO call runs on a tokio pool. Its result returns as a continuation through
  `EventLoopProxy`; the outside world returns as an `Event`.
- Mutations happen only in `dispatch`, a continuation or `apply`. One writer, one thread.
- Render reads `&AppState`. No second copy, no cache nonce, no sync, no `Arc<Mutex<_>>`.
- Periodic work is spawned by the services on the same pool and reports through the same
  channel. Window focus is an input to it.

## Commands, continuations, events

Commands go down as data. Results come back as continuations. Events are the outside
world. One thread applies all three.

- **Command.** An enum grouped by controller, one variant per function, produced by ui
  input, the palette, a keybinding or an MCP tool, never by a direct call.
- **Controller function.** The sync part mutates `AppState` now. The async part is spawned
  through the `Spawner` with the session's cancellation token and carries its continuation.
- **Continuation.** Run on the main thread. It writes the result into the owning service's
  slice and records to the timeline when it should.
- **Generation.** Every job is tagged; a continuation drops a stale result.
- **Event.** External input with no originating command, applied by `apply`: a hook, an
  agent exit, terminal damage, a decision, a poll, a file change, window focus, a config
  change.

**Firehoses never enter the loop.** The terminal's reader thread parses bytes into the grid
and sets a dirty flag; one damage event per frame at most. A hook is one event, and its
redraw merges with the frame's others.

**Keystrokes never touch a device on the main thread.** The terminal module's writer thread
owns the pty's input side; send and resize queue on its channel and return.

**Redraw.** `ControlFlow::Wait`, a dirty flag, one `view(&AppState) → Frame` per batch.

**The one rule the compiler cannot enforce:** no IO in `dispatch`, `apply` or a
continuation.

## Rendering

`Frame` is a display list rebuilt every frame from `&AppState`. Widgets emit it; the
renderer draws it. Neither crate sees the other's types.

- **One glyph per cell** in the terminal and the code surface. No ligatures. A wide
  character takes two cells.
- **Box and block glyphs are quads**, decoded into per-side arm weights that tile exactly.
  Double-line and rounded glyphs stay on the font.
- **Never read the frame back.** The window presents.
- Rounded rects and borders: one SDF fragment shader. Clipping: a scissor per batch.
- Chrome is owned and minimal: about eight primitives on `gfx`, plus focus, hit testing
  and scroll. No general toolkit, no layout engine. `ui-kit` holds `base/` (context,
  tokens, styles, marks, motion), `shape`, `text` and `widgets/` on plain data; it depends
  on `gfx` and `types` only. `ui` adds the hits and layout, `components/` that know
  Groove's state, then `views/`.

Stack: wgpu 30 · glyphon 0.12 · alacritty_terminal 0.26 · ropey · tree-sitter ·
imara-diff.

## Tests

| Layer | How |
|---|---|
| base | each crate standalone: an in-memory database; a mock server for `http` with redaction asserted on the wire; a scripted binary for `exec`; golden images from `Frame` fixtures for `gfx` |
| modules | each against the base: `forge` and `provider` on a fake `http`; `terminal` asserts the grid from a byte stream; `text` pure; `git` on temp repos with repo-local identity and no signing |
| services | each in isolation, on fakes of its modules |
| controllers | drive the API headlessly, assert `AppState`; a `SyncSpawner` runs the job and its continuation at once |
| ui | assert the `Frame` against a fixture `AppState`; no GPU |

A test states a behaviour we want, one behaviour each, named as the sentence it asserts.
Never "the bug stays fixed", never where a thing sits or how it is spaced. A measurement
harness is `#[ignore]`d and named `time_*`.

The layer test in `controllers` reads every manifest and refuses a dependency that points
up or across the services; it also holds the ceilings above. `ui/tests/structure.rs` holds
the shape of `ui` and `ui-kit`: numbers only in the kit's `base/tokens.rs`, styles only in its
`base/style.rs`, and a view draws through the context.

Golden images run on software Vulkan (`mesa-vulkan-drivers`, lavapipe) in CI. The fonts are
vendored: IBM Plex Sans and IBM Plex Mono, OFL.

- [ ] Prove lavapipe on `ubuntu-latest`.

## Migration

The base and `types` first, then the spine, then one vertical slice at a time. A module
lands with its first consumer and only as wide as that consumer needs.

1. Base and shared: `db`, `http`, `exec`, `gfx`, `types`.
2. The spine: the five services as empty slices, the controllers crate with `AppState`,
   `Command`, `dispatch`, `Event`, `apply` and the `Spawner`; `ui` with the rail and an
   empty session surface; the `groove` binary with the loop.
3. Slices, each ending in the window:
   1. a session with its agent — `terminal`, `agent-launch`, the agent pane
   2. sessions and worktrees — `worktree`, the provisioning part of `git`, the rail
   3. the workspace diff — the rest of `git`, `diff`, `text`, `grep`, the code surface in
      three views, the files and diff tabs, the editor on the same surface
   4. the board and tasks — `provider`, the three columns, the filter
   5. MR and forge — `forge` on both hosts, the header's MR and CI, the review column
      and its sessions
   5b. files and the editor — the `file` tab with its three modes, the explorer tree,
      the path operations
   6. asks — `tools`, `mcp-server`, `hooks`, `activity`, `approvals`, `timeline`
   7. settings — `config`, `skills`, `watch`; the keymap gets its pass here
   8. modal editing — a normal mode over the selections the editor already keeps

## Removed at parity

Tauri, ts-rs and `generated/`, the event-name test, `eslint/boundaries.js`, stylelint, the
CSS token system, vitest, vite, `scripts/check-css-classes.mjs`.
