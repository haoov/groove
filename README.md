# Groove

A desktop workspace for task-driven development. One window holds the task, its
repos, its branches, its diff, its merge request and an agent that can act on all of
them — so a piece of work stays in one place instead of a browser, a terminal and an
editor that know nothing about each other.

Groove reads your work from a Notion database and keeps a pool of git worktrees on
disk. Opening a task checks out a branch in every repo the task touches; finishing it
puts the worktrees away.

---

## What it does

**Tasks come from Notion.** Home lists what is assigned to you, ordered by status,
alongside the merge requests waiting on your review. Opening one provisions a git
worktree per repo, on a branch named after the task, and every panel from then on is
scoped to it.

**Sessions.** A *task* session is the normal case. A *review* session checks out
someone else's MR against its real target branch. An *explorer* session is a scratch
checkout with no task behind it, which can be turned into a Notion task later. The
dock on the right lists them all, with what each agent is doing.

**Source control, per task.** Changed files, staging, commit, push, rebase, and the
commit log with a fuzzy filter. Diffs compare against the *merge base*, so you see
what your branch introduced and not what upstream did meanwhile. Context expands per
gap, blame is a gutter toggle (`Alt+B`), and any line can carry an annotation.

**Merge requests.** Create, edit, approve, and read review threads inline — resolve
them, reply to them, and see CI status. Clicking a thread opens the file at the line
it is about. GitLab goes through `glab` and GitHub through `gh`, with the same
features on both; the review queue lists what is waiting on you from both at once.

**An agent that shares the workspace.** The console runs Claude Code inside the
session, and Groove exposes the workspace to it over MCP: the task, its diff, its
commits, its annotations, its MR. The agent reads all of that and proposes actions.

**Approvals.** Anything the agent does that leaves the machine or rewrites history
stops for you first — `git` commit, push, pull, rebase, discard; MR create, update,
close; and every Notion write. Nothing in that list happens without a click.

**Annotations.** Notes pinned to a line, on your side, whether or not an MR exists
yet. The agent can leave them too, and its comments are marked `[claude]`.

**An editor, not an IDE.** CodeMirror with syntax highlighting, Vim mode, file search
and content search. It exists so a two-line fix does not need a context switch — not
to replace your editor.

---

## Requirements

| | Needed for |
|---|---|
| **git** ≥ 2.30 | Required. Worktrees, diffs, commits. |
| **Claude Code** (`claude`) | Required. The agent console and the MCP tools. |
| **curl** | Agent status (waiting / working / idle) in the dock. |
| **glab**, authenticated | GitLab merge requests, threads, CI status. |
| **gh**, authenticated | GitHub pull requests, threads, CI status. |
| **wl-clipboard** / **xclip** / **xsel** | Copying out of the terminal panes. |
| **libnotify** (`notify-send`) | Desktop notifications while the window is unfocused. |

Linux is the supported platform (Wayland and X11). None of this is checked at build
time: the app reports what is missing on first run, and again under
**Settings → This machine**.

---

## Install

### As a package

```sh
pnpm install
pnpm tauri build --bundles deb        # or: rpm, appimage
sudo apt install ./src-tauri/target/release/bundle/deb/Groove_*_amd64.deb
```

### From source

```sh
pnpm install
pnpm tauri dev
```

Building needs the Rust toolchain and the usual Tauri 2 system libraries
(`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `librsvg2-dev`, `patchelf`,
`build-essential`) — see
[tauri.app/start/prerequisites](https://tauri.app/start/prerequisites/).

---

## First run

Groove opens a setup screen when it has no config. It asks for four things, and
reads everything else off the database:

1. **Notion integration token** — notion.so/my-integrations → your integration →
   *Internal Integration Secret*. The task database must be shared with that
   integration (open the database → *…* → *Connections* → add it), or every read
   comes back empty.
2. **Task database id** — open the database as a full page; the id is the last path
   segment before `?v=`.
3. **You, in Notion** — picked from the workspace's people once the token is
   accepted. Used to show only your tasks and to assign the ones you file. Leave it
   unset to see the whole database.
4. **Worktree root** — the directory Groove owns. Point it at an empty directory
   unless you already use the layout below.

Optionally, a **task template page id**: the page whose body seeds a new task. It is
required to turn an explorer session into a task, and it is checked when you save, so
a page the integration cannot read fails there rather than weeks later.

That writes `~/.config/com.haoov.groove/workbench.config.json`; the
screen prints the exact path.

### On-disk layout

```
<worktree_root>/
├── MAIN/                       # the clone pool, fetched and fast-forwarded for you
│   └── <group>/<project>/      # e.g. wiremind/platform/some-service
├── TASKS2-1234/                # one directory per open task
│   ├── some-service/           # a git worktree on branch TASKS2-1234
│   └── another-service/
└── explorer-3/                 # scratch sessions, detached
```

Repos are discovered on disk: anything under `MAIN/` is offered when you add a repo
to a task. Cloning something new is done from the app (`Alt+Shift+R`).

### Notion property names are detected, not configured

The setup screen reads the database's schema and works out which property is which,
then shows you what it found before saving. Nothing here is a name you have to
supply:

- **Type first, name second.** The status property is the one of type `status`,
  whatever it is called. The name only breaks ties — a database with both `Assignee`
  and `Reporter` (two people properties) needs it; a database that calls the field
  `Owner` resolves anyway, because it is the only one of that type.
- **Status meanings come from Notion's own groups.** A status property classifies its
  options into To-do / In progress / Complete, so "filing", "picking up" and
  "finishing" resolve without guessing. That matters where names cannot help: a
  Complete group holding `Fixed with required action`, `Done`, `Abandoned` and
  `Archived` is four completions, and only one of them is what finishing sets.
- **Absent means absent.** No priority property yields no priority pill, rather than
  a name that will 400 the next query.

The detected values are written to the config file and read from there afterwards, so
a wrong detection is corrected by editing one line — not by a rebuild. Reading a
status needs no configuration at all: whatever label Notion returns is classified in
`lib/taskStatus.ts`.

One optional key has no default, being a per-workspace page id: `default_project_id`,
a Project relation set on tasks filed from an explorer session.

### Forge authentication

Each forge is reached through its own CLI, which owns its credentials:

```sh
glab auth login      # GitLab
gh auth login        # GitHub
```

Groove holds no forge token of its own. A repo's host decides which CLI is used, so a
session mixing GitLab and GitHub repos works with no extra setup.

---

## Keyboard

Every binding is editable in **Settings → Keyboard shortcuts**, and matches the
character you type, so non-QWERTY layouts keep working.

| Chord | Does |
|---|---|
| `Alt+Shift+:` | Command palette |
| `Alt+F` | Find file (search bar) |
| `Alt+Shift+F` | Search in files |
| `Ctrl+,` | Open settings |
| `Alt+E` | Files tree |
| `Alt+G` | Source control |
| `Ctrl+Shift+A` | Annotations |
| `Ctrl+Tab` | Cycle git sub-mode |
| `Alt+Shift+C` | Write a commit message |
| `Alt+T` | Home |
| `Ctrl+N` | Notifications |
| `Alt+Shift+N` | Next session tab |
| `Alt+S` | Sessions dock (open / focus) |
| `Alt+Shift+P` | Previous session tab |
| `Alt+'` | Terminal dock (open / focus / close) |
| `Alt+A` | Agent console (open / focus) |
| `` Alt+Shift+\| `` | Split pane right |
| `Alt+-` | Split pane down |
| `Alt+Shift+W` | Close pane |
| `Alt+O` | Focus next pane |
| `Alt+M` | Maximize / restore pane |
| `Alt+N` | Next file tab |
| `Alt+P` | Previous file tab |
| `Alt+W` | Close file tab |
| `Alt+R` | Switch repo |
| `Alt+Shift+R` | Add a repo to this session |
| `Alt+C` | Focus editor |
| `Alt+Shift+V` | Toggle Vim mode |
| `Alt+B` | Toggle blame gutter |

Panels are three-state: the shortcut opens, then focuses, then closes. Lists take
`j`/`k` and `Enter`. Terminals live in one dock at the bottom — splitting it gives
another terminal, side by side.

---

## How it works

```
┌──────────────── Groove (Tauri 2) ─────────────────┐
│  React + TypeScript           Rust                │
│  ──────────────────           ────                │
│  panes, editor, diff   ⇄IPC⇄  git worktrees       │
│  sessions, dock               Notion client       │
│  approvals UI                 glab / GitHub       │
│                               SQLite (state)      │
│                               PTYs (agent, term)  │
│                               MCP server ─────────┼──► Claude Code
└───────────────────────────────────────────────────┘
```

- **State** lives in SQLite under `~/.local/share/com.haoov.groove/`:
  tasks, repos, worktrees, MRs, annotations, tracked time and pending approvals.
  Deleting it loses annotations and time tracking; everything else is re-derivable.
- **The MCP server** listens on `127.0.0.1:27413` and its tools reach the agent as
  `mcp__groove__*` — the prefix a permission allowlist or a hook matcher has to use. Each agent connection is pinned to
  the task it was spawned for, so a tool call cannot address the wrong worktree. If
  the port is busy (usually a second Groove) the app says so instead of failing
  silently.
- **Agents always start at the worktree root**, never inside a repo, so prompts name
  their task explicitly.
- **Writes go through one approval bridge.** Requests survive a crash and re-surface
  on restart.

### Repository layout

```
src/                     React app
  components/            panes, editor, diff, dock, home
    cm/                  CodeMirror shared pieces
  store/                 zustand: app state + per-session reducers
  lib/                   pure logic (layout tree, keymap, diff gaps, matching)
src-tauri/src/
  git_engine/            worktrees, diff, blame, base refs, status, ops
  task_manager/          Notion sync, config, setup, sessions, repos
  mr_manager/            GitLab + GitHub clients
  agent_manager/         PTYs for the agent and terminals
  mcp_server/            the tools agents call
  confirmation_bridge/   approvals
  db/                    schema + row loads
```

### Tests

```sh
pnpm test                      # frontend unit tests (vitest)
cd src-tauri && cargo test     # backend
pnpm exec tsc --noEmit         # types
pnpm exec eslint src           # lint
```

The suites cover pure logic: the pane tree, keymap resolution and migration, diff
parsing and gap arithmetic, base-ref resolution against a real temporary repo, the
blame parser, config serialization. There are no UI tests.

---

## Known limits

- Linux only. Nothing in the code is Linux-specific by design, but nothing else has
  been run.
- One Groove at a time: the MCP port and the SQLite database are both single-owner.
- The agent is Claude Code; there is no other backend.
- The bundle identifier is `com.haoov.groove` and should stay that way: it decides
  where the config and the database live, so changing it moves both. A one-time
  migration carries an older install's state forward, and nobody installing from
  here on needs it.
