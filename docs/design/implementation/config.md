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
| `config.check_environment` | git, gh, glab, claude: version, auth status, a mark |
| `config.login` | open the CLI sign-in in a terminal |
| `config.set_worktree_root` | the one path the user changes |

The Setup section of Settings is the check, the login and the three paths.

## Providers

**Module `config`** holds the source's fields; **`forge`** keeps the token reads. The
Providers section: the task source with its fields under it, then the six properties it
maps — status, priority, start, due, estimate, logged — each with the source's own name
or a gap, and what the gap costs. Forge tokens show present or missing.

| Still to build | Does |
|---|---|
| `config.set_task_source` | turn a source on or off; detect its property names — database, user, GitHub preview as its steps |
| `config.set_source_field` | one field of the active source |

The token stays out of every view and log.

## Appearance

The same four, read by `ui` and `gfx` at startup and on change: the theme picks the
palette, the fonts load through the glyph atlas, the size sets the cell. An empty
family means the bundled default.

| Still to build | Does |
|---|---|
| `config.list_fonts` | read: the system's font families |
| `config.set_theme` · `config.set_ui_font` · `config.set_agent_font` · `config.set_font_size` | persist, republish, redraw |

## Preferences

| Setting | Read by |
|---|---|
| auto-approve default | `agent`, when a session opens |
| attention thresholds — review waiting, due soon, approved unmerged, in days | the `task` service's attention fold |
| poll interval and stale threshold | the `workspace` service's poll |
| git: clone pool path | `worktree` |

| Still to build | Does |
|---|---|
| `config.set_preference` | one named preference; persist, republish |

Every change saves on the spot. No save button; no restart.

## Needs

- [x] The config file's fields: auto-approve default, thresholds, poll interval.
- [ ] Settings search: an index of every row's label and section, built from the form.
- [ ] The clipboard's home: `ui` through winit, since it is the window's, not a service's.
