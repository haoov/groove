# Config — implementation

How the tool sets in [../capabilities/config.md](../capabilities/config.md) are made,
in the words of [../architecture.md](../architecture.md). Each table lists the actions
still to build; for the rest the `Command` enum is the truth.

## Setup

**Module `config`**: the files the app keeps beside the database, and the environment
check through `exec::run`. The user writes `~/.config/groove/config.json`; the app writes
the window's boundaries to `~/.local/share/groove/panes.json`. **Service `config`** holds
the parsed config and the last check. The state database is
`~/.local/share/groove/app.db`, so the legacy app's files stay untouched until parity.

| Still to build | Does |
|---|---|
| `config.get` | read: the config as the ui sees it, token excluded |
| `config.write_initial` | first run: worktree root and sources; detects the Notion property names |
| `config.set_worktree_root` | moves the pool and every worktree; the root stays read only until then |

The Setup section of Settings is the three paths, then the check, then the login.
`config.check_environment` runs each time Settings opens: git, claude, curl, glab and gh,
each with its version and, for claude, glab and gh, its sign-in. `config.login` runs
`claude auth login` on a terminal the `agent` service owns, in the lower half of Setup.
It takes the keys and the clipboard while it runs, and the check runs again when it exits.

The agent needs no MCP setup: each launch passes the app's loopback through `--mcp-config`.

## Providers

**Module `config`** holds the source's fields; **`forge`** keeps the token reads. The
Providers section: the task source with its fields under it, then the six properties it
maps — status, priority, start, due, estimate, logged — each with the source's own name
or a gap, and what the gap costs. Forge tokens show present or missing, as the environment
check found the `gh` and `glab` sign-in.

`config.set_task_source` turns a source on or off. Turning it on only proves Groove can
reach it: Notion's token must read the database, GitHub's host must answer the `gh` token.
Notion needs a token, a database id and your user id: Groove lists only your tasks. The
source starts with every name a gap; the mapping is its own step. Notion's assignee and
sprint are required: until both are named, it lists no task and files none. Turning a source off
asks once more, removes its block, and reads the tasks again; the last source stays on.

`config.read_schema` reads what a source holds, once, while Providers shows it: Notion's
database properties, GitHub's fields on the boards your issues stand on. `config.map`
changes one thing and reads the tasks again. A name is picked from the properties of its
type: a date for start and due, a number for estimate and logged, people for the
assignee, a relation for the sprint. The mapping runs from what Groove uses: under the
status, ready, in progress and done each take one value of the source's status; under the
priority, high, medium and low each take one value. A name pointed elsewhere forgets its
values. The statuses left out of the list stay in the file, `filters.exclude_statuses`.

The token stays out of every view and log.

## Appearance

The same four, read by `ui` and `gfx` at startup and on change: the theme picks the
palette, the fonts load through the glyph atlas, the size sets the cell. An empty
family means the bundled default.

The theme is a preference: `config.set_preference` with `Theme` saves it, the next frame
draws in it, and every running agent and shell terminal takes its colours.

Three sizes, each a `FontSize` preference stepped by one point: the interface, the editor
(code and diff), and the terminals (agent and shells). An unset terminal size is the
editor's. The next frame lays out at the new size and the terminals resize to it.

| Still to build | Does |
|---|---|
| `config.list_fonts` | read: the system's font families |
| `config.set_ui_font` · `config.set_agent_font` | persist, republish, redraw |

## Preferences

| Setting | Read by |
|---|---|
| auto-approve default | a new session, as it goes on the rail |
| attention thresholds — review waiting, due soon, approved unmerged, in days | the `task` service's attention fold |
| poll interval, no shorter than ten seconds | the `delivery` poll |
| stale threshold: an MR read longer ago reads as old, as a failed read does | the `delivery` poll |
| git: clone pool path | `worktree`, set from Setup |

`config.set_preference` carries one named preference. The config service applies it, every
reader sees it at once, and the file is written on the spot, as `panes.json` is. No save
button; no restart.

## Keymap

One table in `ui` holds every action with its group, its ring and its default chords; the
palette and Settings read their labels from it. Three rings, heard in this order:

| Ring | Modifier | Heard |
|---|---|---|
| app | `alt`, `alt+shift` | from any pane; in a terminal only these chords are taken, every other alt key passes through |
| code | `ctrl`, `ctrl+shift` | by the focused pane's content: the editor, the diff, find, the commit box |
| terminal | `ctrl+shift+c`, `ctrl+shift+v` | in the agent and the shells, which take every other key |

A chord matches the key as it reads without a modifier, so it stays on its key whatever the
layout, AltGr or Option make of it. The defaults stay clear of the Alt keys Claude Code
binds (p, o, t, v, up, down) and of readline's (b, f, d, the digits, the arrows). The file's
`keymap` block holds only what is rebound: an action's id to its chords. In Settings a chord
is rebound by pressing it; it needs ctrl or alt, and it comes off whatever action held it.

## Needs

- [x] The config file's fields: auto-approve default, thresholds, poll interval.
- [x] Settings search: an index of every row's label and section, built from the form.
- [x] The Settings surface: Preferences, the theme and the font sizes set in place; the rest read only.
- [ ] The clipboard's home: `ui` through winit, since it is the window's, not a service's.
