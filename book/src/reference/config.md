# Config file

Groove reads `~/.config/groove/config.json`, or `$XDG_CONFIG_HOME/groove/config.json`.
Settings writes every change to it at once. You can also edit it by hand: keep it valid
JSON. Groove keeps the file readable only by you, because it can hold a Notion token.

Groove creates the file at the first launch. Every key has a default.

## `git`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `worktree_root` | string | `~/groove` | the folder that holds the clones (`main/`) and the worktrees (`worktrees/`). `~` is expanded. |

## `ui`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `theme` | string | `latte` | `latte`, `frappe`, `macchiato` or `mocha` |
| `font_size` | number | `13` | interface text size, 8 to 32 |
| `code_font_size` | number | `12.5` | editor and diff text size, 8 to 32 |
| `terminal_font_size` | number | the code size | terminal text size, 8 to 32 |
| `font_family` | string | `""` | interface font; empty is IBM Plex Sans |
| `mono_font_family` | string | `""` | `""` (IBM Plex Mono), `JetBrains Mono` or `Lilex` |
| `wrap_code` | boolean | `false` | wrap long lines of code |

## `preferences`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `auto_approve_default` | boolean | `false` | the auto-approve value of a new session |
| `poll_interval_secs` | number | `60` | seconds between two reads of the MRs, 10 minimum |
| `stale_after_secs` | number | `300` | an MR read older than this shows as stale |
| `routine_cap` | number | `5` | the most agents that run routines at once, 1 minimum |
| `routines_paused` | boolean | `false` | pause every routine |
| `thresholds` | object | see below | when an item asks for your attention |

`thresholds` holds `review_waiting_days` (3), `due_soon_days` (2),
`approved_unmerged_days` (1) and `ci_failed_minutes` (10).

## `github`

The GitHub issues task source. Remove the block to turn it off.

| Key | Meaning |
|---|---|
| `host` | the GitHub host, such as `github.com` |
| `token` | optional; else Groove asks `gh auth token` |
| `properties`, `status_map`, `priority_map` | the mapping Settings › Providers writes |

## `notion`

The Notion task source. Remove the block to turn it off.

| Key | Meaning |
|---|---|
| `token` | the secret of the Notion integration |
| `database_id` | the id of the task database |
| `user_id` | your Notion user id |
| `assignee` | the people property that marks a task as yours |
| `sprint` | the relation to the sprint |
| `sprint_status` | the status of the current sprint; default `Current` |
| `properties` | the property names: `status`, `priority`, `start`, `due`, `estimate`, `estimate_unit` (`hours` or `days`, 8 hours a day), `logged`, `project` |
| `status_map` | the values for `ready`, `in_progress` and `done`; Groove writes the first one |
| `priority_map` | the values for `high`, `medium` and `low` |
| `filters.exclude_statuses` | the statuses Groove does not list |
| `task_template_page_id` | the page a new task copies |
| `default_project_id` | the project a new task gets |

Notion lists nothing until `assignee` and `sprint` are set.

## `clusters`

A list, one object for each context you added in Settings › Clusters.

| Key | Meaning |
|---|---|
| `context` | the kubeconfig context name |
| `hue` | `sapphire`, `mauve`, `pink`, `teal`, `flamingo` or `sky` |

## Other keys

| Key | Meaning |
|---|---|
| `keymap` | the keys you changed; see [Keymap](keymap.md) |
| `shared` | the team's shared repo: `url`, `branch`, and `enabled`, the shared skills you turned on |
| `skills_off` | the ids of your own skills that sessions do not get |
| `routines` | `on`, the routines you turned on, and `quiet`, the triggers muted on each |

## Other files

| Path | Holds |
|---|---|
| `~/.config/groove/user-skills/` | your own skills |
| `~/.config/groove/routines/` | your own routines |
| `~/.local/share/groove/app.db` | sessions, notes, the timeline |
| `~/.local/share/groove/panes.json` | the pane widths |
| `~/.cache/groove/` | the cache |
