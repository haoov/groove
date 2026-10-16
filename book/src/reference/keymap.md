# Keymap

These are the default keys. On macOS they are the same: `alt` is Option, and Groove uses
no Cmd key.

## Change a key

Open Settings › Keymap, select an action and press the new chord. A chord needs `ctrl` or
`alt`. The chord leaves any other action that held it. A key matches the same physical
key on every keyboard layout.

Groove saves only the keys you change, in the `keymap` key of `config.json`: an action id
and the list of chords that replace its defaults.

```json
{
  "keymap": { "palette.open": ["alt+p"] }
}
```

## Everywhere

| Key | Action id | Does |
|---|---|---|
| `alt+k` | `palette.open` | open or close the palette |
| `alt+,` | `settings.open` | open Settings |
| `alt+h` | `board.toggle` | open the board, or go back |
| `alt+shift+n` | `session.open_explorer` | open a new explorer |
| `alt+shift+w` | `session.close` | close the session |
| `alt+shift+down` | `session.next` | next session |
| `alt+shift+up` | `session.previous` | previous session |
| `alt+shift+left` | `pane.left` | pane to the left |
| `alt+shift+right` | `pane.right` | pane to the right |
| `alt+1` | `pane.rail` | the rail |
| `alt+2` | `pane.agent` | the agent pane |
| `alt+3` | `pane.workspace` | the workspace |
| `alt+4` | `pane.sidebar` | the sidebar |
| `alt+c` | `pane.commit` | the commit box |
| `alt+]` | `tab.next` | next tab |
| `alt+[` | `tab.previous` | previous tab |
| `alt+shift+b` | `sidebar.fold` | fold the sidebar |
| `alt+shift+1` | `tab.overview` | Overview tab |
| `alt+shift+2` | `tab.diff` | Diff tab |
| `alt+shift+3` | `tab.files` | Files tab |
| `alt+shift+4` | `tab.resources` | Kubernetes tab |
| `alt+'` | `terminals.toggle` | show or hide the terminals |
| `alt+shift+'` | `terminals.new` | new terminal |
| `alt+shift+r` | `workspace.load` | reload the workspace |

On the board, `alt+1` is the rail, and `alt+2` to `alt+4` are the Live, Up next and
Review columns.

## Open tabs

| Key | Action id | Does |
|---|---|---|
| `ctrl+w` | `open.close` | close the open tab |
| `ctrl+tab`, `ctrl+pagedown` | `open.next` | next open tab |
| `ctrl+shift+tab`, `ctrl+pageup` | `open.previous` | previous open tab |

## Code and lists

| Key | Action id | Does |
|---|---|---|
| `ctrl+s` | `editor.save` | save |
| `ctrl+z` | `editor.undo` | undo |
| `ctrl+shift+z`, `ctrl+y` | `editor.redo` | redo |
| `ctrl+a` | `editor.select_all` | select all |
| `ctrl+c` | `editor.copy` | copy |
| `ctrl+x` | `editor.cut` | cut |
| `ctrl+v` | `editor.paste` | paste |
| `ctrl+f` | `find.open` | find in the file or the list |
| `ctrl+n` | `find.next` | next match |
| `ctrl+p` | `find.previous` | previous match |
| `ctrl+shift+p` | `files.open_path` | open a file by path, or search the kinds |
| `ctrl+shift+f` | `files.search` | search in files |
| `ctrl+shift+n` | `resources.select_namespace` | select a namespace |
| `ctrl+enter` | `commit` | commit |

## Terminals

| Key | Action id | Does |
|---|---|---|
| `ctrl+shift+c` | `terminal.copy` | copy |
| `ctrl+shift+v` | `terminal.paste` | paste |

Every other key goes to the shell or the agent.

## Keys you cannot change

- **Board**: Up and Down select a row. Enter opens it. `ctrl+enter` opens it and stays on
  the board. Esc lets go of the column.
- **Note**: Enter sends. `shift+enter` adds a line. Esc cancels.
- **Settings**: Esc goes back.
