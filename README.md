# Groove

A desktop workspace for task-driven development. One window holds the task, its
repos, its branches, its diff, its merge request and an agent that can act on all of
them — so a piece of work stays in one place instead of a browser, a terminal and an
editor that know nothing about each other.

Groove reads your work from Notion or GitHub and keeps a pool of git worktrees on
disk. Opening a task checks out a branch in every repo the task touches; finishing it
puts the worktrees away.

<!-- SCREENSHOT: the whole window on a task session — editor + diff + agent console.
     Landscape, ~1600px wide. This is the one that has to sell the idea. -->
![Groove on a task session](docs/images/workspace.png)

---

## What it does

**Tasks come from Notion or GitHub.** Home lists what is assigned to you, ordered by
status, alongside the merge requests waiting on your review. Opening one provisions a
git worktree per repo, on a branch named after the task, and every panel from then on
is scoped to it.

<!-- SCREENSHOT: Home, Live tab, a session or two expanded, filter bar in use. -->
![Home](docs/images/home.png)

**Sessions.** A *task* session is the normal case. A *review* session checks out
someone else's MR against its real target branch. An *explorer* session is a scratch
checkout with no task behind it, which can be turned into a real task later. Header
pickers switch session, repo and worktree.

**Filter, don't hunt.** Home's search bar takes `field:value` — `provider:github`,
`priority:high`, `-kind:explorer`, `title:"fix the parser"`. Repeat a key for OR,
quote a value with spaces, press `/` to jump to the bar. It autocompletes from your
real data, and a committed query switches to whichever tab can answer it.

**Source control, per task.** Changed files (flat or tree), staging, commit, push,
rebase, and the commit log with a fuzzy filter. Diffs compare against the *merge
base*, so you see what your branch introduced and not what upstream did meanwhile.
Context expands per gap, blame is a gutter toggle (`Alt+B`), and any line can carry an
annotation.

<!-- SCREENSHOT: the diff view with a hunk expanded and an annotation on a line. -->
![Diff and annotations](docs/images/diff.png)

**Merge requests.** Create, edit, approve, and read review threads inline — resolve
them, reply to them, and see CI status. Clicking a thread opens the file at the line
it is about. GitLab goes through `glab` and GitHub through `gh`, with the same
features on both; the review queue lists what is waiting on you from both at once.

**An agent that shares the workspace.** The console runs Claude Code inside the
session, and Groove exposes the workspace to it over MCP: the task, its diff, its
commits, its annotations, its MR. The agent reads all of that and proposes actions.

<!-- SCREENSHOT: the agent console mid-turn, with an approval dialog up. -->
![Agent console and an approval](docs/images/agent.png)

**Approvals.** Anything the agent does that leaves the machine or rewrites history
stops for you first — `git` commit, push, pull, rebase, discard; MR create, update,
close; and every task write. Nothing in that list happens without a click, unless you
tell one session to allow everything: that is a switch in the agent's own action bar,
scoped to that session, and gone when it closes.

**Annotations.** Notes pinned to a line, on your side, whether or not an MR exists
yet. The agent can leave them too, and its comments are marked `[claude]`.

**An editor, not an IDE.** CodeMirror with syntax highlighting, Vim mode, file search
and content search. It exists so a two-line fix does not need a context switch — not
to replace your editor.

---

## Install

Download a build from [Releases](https://github.com/haoov/groove/releases), or build
from source.

```sh
# Linux
sudo apt install ./Groove_0.2.0_amd64.deb        # or the .rpm
chmod +x Groove_0.2.0_amd64.AppImage && ./Groove_0.2.0_amd64.AppImage
```

### From source

```sh
pnpm install
pnpm tauri dev                                   # or: pnpm tauri build
```

Building needs the Rust toolchain, plus on Linux the usual Tauri 2 system libraries
(`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `librsvg2-dev`, `patchelf`,
`build-essential`) — see
[tauri.app/start/prerequisites](https://tauri.app/start/prerequisites/). macOS needs
only the Xcode Command Line Tools (`xcode-select --install`).

Bundle targets: `pnpm tauri build --bundles deb` (or `rpm`, `appimage`, and on macOS
`app,dmg`). A local macOS build is ad-hoc signed — fine on the machine that built it,
refused by Gatekeeper anywhere else; see [Releasing on macOS](#releasing-on-macos).

## Requirements

Linux (Wayland and X11) and macOS 10.15+.

| | Needed for | |
|---|---|---|
| **git** ≥ 2.30 | Required. Worktrees, diffs, commits. | both |
| **Claude Code** (`claude`) | Required. The agent console and the MCP tools. | both |
| **curl** | Agent status (waiting / working / idle). | both |
| **glab**, authenticated | GitLab merge requests, threads, CI status. | both |
| **gh**, authenticated | GitHub pull requests and threads. Also GitHub tasks, which additionally need the `project` scope. | both |
| **wl-clipboard** / **xclip** / **xsel** | Copying out of the terminal panes. | Linux |
| **libnotify** (`notify-send`) | Desktop notifications while unfocused. | Linux |

The last two have no macOS entry because there is nothing to install: `pbcopy` and
`osascript` ship with the OS. Claude Code's **CLI** is what is needed — a separate
install from the desktop app. On macOS the rest is `brew install git gh glab`.

None of this is checked at build time: the app reports what is missing on first run,
and again under **Settings → This machine**, where it also says whether each forge CLI
is signed in and offers to run `glab auth login` / `gh auth login` in a terminal
inside the app.

---

## First run

Groove opens a setup screen when it has no config. Turn on at least one task source
and set a worktree root; everything else is read off the source.

<!-- SCREENSHOT: the first-run screen with both sources switched on. -->
![First run](docs/images/first-run.png)

**GitHub** needs nothing typed — `gh` already holds the credential. Switching it on
shows what it can see: how many issues assigned to you sit on a board, which boards,
each board's status columns, and how many issues are on no board and will be skipped.

**Notion** asks for four things:

1. **Integration token** — notion.so/my-integrations → your integration →
   *Internal Integration Secret*. The task database must be shared with that
   integration (open the database → *…* → *Connections* → add it), or every read comes
   back empty.
2. **Task database id** — open the database as a full page; the id is the last path
   segment before `?v=`.
3. **You, in Notion** — your email, once the token is accepted. Pasting a user id
   works too, but check it is a *user* id: a page id has the same shape, and filtering
   on one silently matches no tasks. Leave it empty to see the whole database.
4. Optionally a **task template page id** — the page whose body seeds a new task.
   Required to turn an explorer session into a Notion task, and checked when you save,
   so a page the integration cannot read fails there rather than weeks later.

**Worktree root** — the directory Groove owns. Point it at an empty directory unless
you already use the layout below.

Sources can be added or removed later under **Settings → Task sources**; the setup
screen is not reachable again once a config exists. Config is written to
`~/.config/com.haoov.groove/workbench.config.json` — the screen prints the path.

### On-disk layout

```
<worktree_root>/
├── main/                             # the clone pool, fetched and fast-forwarded for you
│   ├── gitlab.example.com/
│   │   └── <group>/<project>/        # e.g. wiremind/platform/some-service
│   └── github.com/
│       └── <owner>/<project>/
└── worktrees/
    ├── TASKS2-1234/                  # one directory per open session
    │   └── some-service/feat/parser  # <project>/<branch> — the branch keeps its slashes
    └── explorer-3/
```

The forge host is a directory of its own, so two instances can hold the same group and
project name, and the pool says where each clone came from. Repos are discovered on
disk: anything under `main/` is offered when you add a repo to a session. Clone
something new from the app (`Alt+Shift+R`).

### Task sources

Notion reads a database you point it at. GitHub needs no configuration beyond being
switched on: a task is an **open issue assigned to you that sits on a Projects v2
board**, and the board's own fields (Status, Priority, whatever else) become the
task's properties. An issue on no board is deliberately not a task — that is the
filter.

Both can be on at once; their tasks share one queue. GitHub needs `gh` signed in with
the `project` scope (`gh auth refresh -s project`) — the setup screen reports it and
offers to run it for you.

### Notion property names are detected, not configured

The setup screen reads the database's schema, works out which property is which, and
shows you what it found before saving:

- **Type first, name second.** The status property is the one of type `status`,
  whatever it is called. The name only breaks ties — a database with both `Assignee`
  and `Reporter` needs it; one that calls the field `Owner` resolves anyway, because
  it is the only property of that type.
- **Status meanings come from Notion's own groups.** A status property classifies its
  options into To-do / In progress / Complete, so "filing", "picking up" and
  "finishing" resolve without guessing. That matters where names cannot help: a
  Complete group holding `Fixed with required action`, `Done`, `Abandoned` and
  `Archived` is four completions, and only one is what finishing sets.
- **Absent means absent.** No priority property yields no priority pill, rather than a
  name that will 400 the next query.

Detected values are written to the config file and read from there afterwards, so a
wrong detection is corrected by editing one line — not by a rebuild.

GitHub boards are not detected: each board names its own columns, so the setup preview
shows them and `github.status_map` in the config file is where you correct a mismatch.
Finishing a task tells you the board's real column names if none match.

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
character you type, so non-QWERTY layouts keep working. The macOS column is filled in
only where the chord differs — four do, because Option composes a character out of the
punctuation they use.

| Chord | macOS | Does |
|---|---|---|
| `Alt+Shift+:` | `⌥⇧K` | Command palette |
| `Alt+F` | | Find file |
| `Alt+Shift+F` | | Search in files |
| `Ctrl+,` | | Settings |
| `Alt+E` | | Files tree |
| `Alt+G` | | Source control |
| `Ctrl+Shift+A` | | Annotations |
| `Ctrl+Tab` | | Cycle git sub-mode |
| `Alt+Shift+C` | | Write a commit message |
| `Alt+T` | | Home |
| `/` | | Focus Home's filter |
| `Ctrl+N` | | Notifications |
| `Alt+S` | | Session picker (repeat to cycle) |
| `Alt+R` | | Repo picker |
| `Alt+W` | | Worktree picker |
| `Alt+Shift+N` / `Alt+Shift+P` | | Next / previous session |
| `Alt+'` | `⌥J` | Terminal dock |
| `Alt+A` | | Agent console |
| `` Alt+Shift+\| `` | `⌥D` | Split pane right |
| `Alt+-` | `⌥⇧D` | Split pane down |
| `Alt+Shift+W` | | Close pane |
| `Alt+O` | | Focus next pane |
| `Alt+M` | | Maximize / restore pane |
| `Alt+N` / `Alt+P` | | Next / previous file tab |
| `Alt+Shift+R` | | Add a repo to this session |
| `Alt+C` | | Focus editor |
| `Alt+Shift+V` | | Toggle Vim mode |
| `Alt+B` | | Toggle blame gutter |

Panels are three-state: the shortcut opens, then focuses, then closes. Lists take
`j`/`k` and `Enter`. Mouse back/forward cycle panes. Terminals live in one dock at the
bottom; splitting it gives another terminal side by side.

### Platform differences

The window is frameless on Linux and draws its own controls; on macOS it is a normal
decorated window with the traffic lights over the header
(`src-tauri/tauri.macos.conf.json`).

Because Option is how macOS types `é` and the rest, an Option chord runs its command
even inside a text field — the same bargain Linux already makes with Alt. Rebind
anything in **Settings → Keyboard shortcuts**.

---

## How it works

```
┌──────────────── Groove (Tauri 2) ─────────────────┐
│  React + TypeScript           Rust                │
│  ──────────────────           ────                │
│  panes, editor, diff   ⇄IPC⇄  git worktrees       │
│  sessions, home               task providers      │
│  approvals UI                 glab / gh           │
│                               SQLite (state)      │
│                               PTYs (agent, term)  │
│                               MCP server ─────────┼──► Claude Code
└───────────────────────────────────────────────────┘
```

- **State** lives in SQLite under `~/.local/share/com.haoov.groove/app.db`: sessions,
  repos, worktrees, MRs, annotations, tracked time and pending approvals. Deleting it
  loses annotations and time tracking; everything else is re-derivable.
- **Task sources sit behind one trait** (`TaskProvider`) with a registry that decides
  what is configured. Notion and GitHub Issues are the two implementations.
- **The MCP server** listens on `127.0.0.1:27413` and its tools reach the agent as
  `mcp__groove__*` — the prefix a permission allowlist or hook matcher has to use.
  Each agent connection is pinned to the session it was spawned for, so a tool call
  cannot address the wrong worktree.
- **Agents always start at the worktree root**, never inside a repo, so prompts name
  their session explicitly.
- **Writes go through one approval bridge.** Requests survive a crash and re-surface
  on restart.

The module map and the invariants worth knowing before changing anything:
**[CLAUDE.md](CLAUDE.md)**.

```sh
pnpm test                          # frontend unit tests
npx tsc --noEmit                   # types
pnpm lint                          # eslint
cd src-tauri && cargo test --lib   # backend
```

The suites cover pure logic: the pane tree, keymap resolution, diff parsing and gap
arithmetic, base-ref resolution against a real temporary repo, the blame parser, the
filter grammar, provider identity and short-id minting, the store's reducers, and the
DB store against an in-memory SQLite. There are no UI tests.

---

## Releasing on macOS

A local `pnpm tauri build` is **ad-hoc signed**: it runs on the machine that built it
and Gatekeeper refuses it anywhere else. Shipping needs an Apple Developer Program
membership, a *Developer ID Application* certificate, and notarization.

Build for both architectures — the default build is whatever the machine is, so an
Apple Silicon build alone leaves Intel Macs out:

```sh
rustup target add x86_64-apple-darwin
pnpm tauri build --target universal-apple-darwin --bundles app,dmg
```

If that fails with `can't find crate for 'core'` naming a target you just added, the
Rust on PATH is not the one rustup manages — a Homebrew `rust` shadows it. Check
`which cargo`; `~/.cargo/bin` has to come first.

Signing and notarization are driven entirely by environment variables; Tauri picks
them up with no config change. Certificate: `APPLE_CERTIFICATE` (the `.p12`, base64
encoded), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, and on CI
`KEYCHAIN_PASSWORD`. Notarization: either an App Store Connect API key
(`APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_PATH`) or an Apple ID
(`APPLE_ID`, `APPLE_PASSWORD` — an app-specific password — and `APPLE_TEAM_ID`).

Then verify, which is the only check that catches a signature that exists but is not
trusted:

```sh
codesign -dv --verbose=4 src-tauri/target/release/bundle/macos/Groove.app
spctl -a -vvv src-tauri/target/release/bundle/macos/Groove.app
```

`spctl` must say *accepted*. Confirm on a Mac that never saw the build — the building
machine trusts its own ad-hoc signature and passes either way.

---

## Known limits

- One Groove at a time: the MCP port and the SQLite database are both single-owner.
- The agent is Claude Code; there is no other backend.
- The bundle identifier is `com.haoov.groove` and should stay that way: it decides
  where the config and the database live, so changing it moves both.
- No CI. The four commands above are the gate.
