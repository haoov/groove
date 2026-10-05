# Config

Set Groove up and keep its preferences. Surface: settings, as
[../design.md](../design.md#settings) draws it.

**Every change saves to the file on the spot.** No save button, no restart; every reader
sees it at once. The user's file is `~/.config/groove/config.json`. Groove's own state is
`~/.local/share/groove/app.db`, and the window's boundaries are
`~/.local/share/groove/panes.json`.

**A token stays out of every view and log.**

## Setup

**The environment check runs each time Settings opens**: git, claude, curl, glab and gh,
each with its version, and the sign-in for claude, glab and gh. The claude login runs in
a terminal inside Setup and the check runs again when it exits.

**The agent needs no MCP setup.** Each launch passes Groove's tool server through
`--mcp-config`.

## Providers

**Turning a source on only proves Groove can reach it**: Notion's token must read the
database, GitHub's host must answer the `gh` token. Notion needs a token, a database id
and the user's id, since Groove lists only the user's tasks. Its assignee and sprint are
required: until both are named, it lists and files nothing.

**The mapping is its own step.** A source starts with every name a gap. Groove reads what
the source holds once while Providers shows it, and offers only properties of the right
type: a date for start and due, a number for estimate and logged, people for the
assignee, a relation for the sprint. Under the status, ready, in progress and done each
take one of the source's values; under the priority, high, medium and low.

**The last source stays on.** Turning a source off asks once more, removes its block and
reads the tasks again.

## Agent

The auto-approve default, which a new session takes; the shared repo; the skills with
their switches; the routines, as [../shared.md](../shared.md) describes them.

## Appearance

**The theme repaints everything at once**, the running agent and shell terminals
included.

**Two families, all vendored.** The UI font is IBM Plex Sans or JetBrains Mono. The mono
font is IBM Plex Mono, JetBrains Mono or Lilex, and draws code, the diff, the agent and
the shells. Mono draws no ligatures. An empty or unknown family is IBM Plex. A change
redraws at once.

**Three sizes**, stepped by one point: the interface, the editor (code and diff), and the
terminals (agent and shells). An unset terminal size is the editor's.

## Preferences

| Setting | Read by |
|---|---|
| attention thresholds — review waiting, due soon, approved unmerged, in days | the task capability's attention fold |
| poll interval, no shorter than ten seconds | the forge poll |
| stale threshold: an MR read longer ago reads as old, as a failed read does | the forge poll |

## Keymap

**One table in `ui` holds every action** with its group, its ring and its default chords.
The palette and Settings read their labels from it. The config's `keymap` block holds
only what the user rebound.

**Three rings, heard in this order:**

| Ring | Modifier | Heard |
|---|---|---|
| app | `alt`, `alt+shift` | from any pane; in a terminal only these chords are taken, every other alt key passes through |
| code | `ctrl`, `ctrl+shift` | by the focused pane: the editor, the diff, find, the commit box |
| terminal | `ctrl+shift+c`, `ctrl+shift+v` | in the agent and the shells, which take every other key |

**A chord matches the key without its modifier**, so it stays on its key whatever the
layout, AltGr or Option make of it.

**The defaults keep clear of Claude Code's and readline's alt keys**: p, o, t, v, up and
down; b, f, d, the digits and the arrows.

**A chord is rebound by pressing it.** It needs ctrl or alt, and it comes off whatever
action held it.
