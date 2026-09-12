# Config — implementation

How the tool sets in [../capabilities/config.md](../capabilities/config.md) are made:
today, from the code; planned, in the words of [../architecture.md](../architecture.md).

## Setup

### Today

`check_environment` resolves each external program against the widened PATH and
reports its version; for `gh` and `glab` it runs `auth status` and reads the scopes;
it names the clipboard tool for the display server. `start_auth_session` opens a shell
for the CLI sign-in; the `claude` login is a PTY of the same kind. `write_initial_config`
takes the worktree root and the sources, detects the Notion property names and writes
the file. The config lives at `~/.config/com.haoov.groove/workbench.config.json`,
the state at `~/.local/share/com.haoov.groove/app.db`; the bundle identifier decides
both. `config::init` loads once at startup; `update` mutates, persists and publishes.

### Planned

**Module `config`**: the file, load, update and persist; the environment check through
`exec::run`. **Service `config`** holds the parsed config and the last check.

| Controller | Does |
|---|---|
| `config.check_environment` | git, gh, glab, claude: version, auth status, a mark |
| `config.login` | open the CLI sign-in in a terminal |
| `config.set_worktree_root` | the one path the user changes |
| `config.open_path` | the config file or the state database, in the system |

The Setup section of Settings is the check, the login and the three paths.

## Providers

### Today

`Config` carries `notion` and `github` blocks. Notion: the token, the database, the
user, the property names, the status map. GitHub: the host, the property names, the
status map; its token comes from `gh auth token`. `ConfigView` is what the frontend
sees — everything except the Notion token — and the token never reaches a `Debug`
rendering. Forge tokens are read from `gh` and `glab` once per host and cached in
memory.

### Planned

**Module `config`** as today; **`forge`** keeps the token reads. The Providers section:
the task source as a select with its fields under it; forge tokens shown present or
missing from `forge`'s check.

| Controller | Does |
|---|---|
| `config.set_task_source` | turn a source on or off; detect its property names |
| `config.set_source_field` | one field of the active source |

The token stays out of every view and log, as today.

## Appearance

### Today

`UiConfig`: font size, theme, UI font family, agent font family. Theme is Catppuccin,
Latte by default; an empty font family means the CSS stack.

### Planned

The same four, read by `ui` and `gfx` at startup and on change: the theme picks the
palette, the fonts load through the glyph atlas, the size sets the cell. An empty
family means the bundled default.

| Controller | Does |
|---|---|
| `config.set_theme` · `config.set_ui_font` · `config.set_agent_font` · `config.set_font_size` | persist, republish, redraw |

## Preferences

### Today

`suggest_actions` hides every skill chip when off. `GitConfig` holds the worktree
root. `FilterConfig` excludes statuses and filters by assignee.

### Planned

| Setting | Read by |
|---|---|
| suggest actions | the overview's chips |
| auto-approve default | `agent`, when a session opens |
| attention thresholds — review waiting, due soon, approved unmerged, in days | the `task` service's attention fold |
| poll interval and stale threshold | the `workspace` service's poll |
| git: clone pool path | `worktree` |

| Controller | Does |
|---|---|
| `config.set_preference` | one named preference; persist, republish |

Every change saves on the spot. No save button; no restart.

## Needs

- [ ] The config file's new fields: auto-approve default, thresholds, poll interval.
- [ ] Settings search: an index of every row's label and section, built from the form.
