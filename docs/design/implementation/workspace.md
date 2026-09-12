# Workspace — implementation

How the tool sets in [../capabilities/workspace.md](../capabilities/workspace.md) are
made: today, from the code; planned, in the words of
[../architecture.md](../architecture.md). Everything here follows the session's
selected worktree.

## Diff

### Today

`review/diff.rs` produces the diff per session, every worktree: repo → files → hunks.
The summary — path, added and deleted counts, status letter, staged — comes first;
`get_file_diff` fills a file's hunks when displayed. Three modes: base, the merge-base
with the base branch, the MR's target for a review; working, uncommitted against
HEAD; vs-remote, the branch's remote tip or the base when unpushed. Untracked files
become all-additions hunks, capped by size and line count; binary and oversized files
are listed, not rendered. A throttled background `git fetch origin` keeps base and
remote fresh. `review/commits.rs` lists the task's own commits against the base
history and one commit's diff. `review/blame.rs` blames a file with uncommitted lines
marked. `read_file_lines` returns any slice of a file's new side, from disk or a
revision, for expansion. `core/git/cache.rs` caches ref answers with a short TTL and
is flushed after every git operation the app runs.

CodeMirror renders it: unified only, syntax by extension, word diff in changed pairs,
gap expansion, gutters for line numbers, blame and annotation markers, search in
file. The file list is a directory tree with status, counts and a staged marker.

### Planned

**Module `diff`**, on `git` and `text`: the three modes, the summary, hunks on demand,
untracked handling, commits, blame, expansion, the fetch throttle. Parsers stay in
`git`. The cache goes; one state is refetched on refresh with a generation.

**Service `workspace`** holds per selected worktree: the summary, the hunks loaded so
far, the view — unified or split — the mode, blame when asked.

**Module `watch`**: a filesystem watcher on the selected worktree, `.git` excluded,
debounced. A change to a file — modified, created, deleted, by the agent, the editor
or a terminal — arrives as `Event::Workspace(FilesChanged { worktree, paths })`; the
service reloads the summary, the changed files and the explorer list under a new
generation. The hooks' refresh on file-editing tools is no longer needed.

**Refresh by hand.** A button on the session header reloads everything for the
selected worktree at once — git status, the summary, MR, CI and threads — without
waiting for the watcher or the poll.

**Rendering** in `ui` on `gfx`: one glyph per cell, syntax from tree-sitter through
`text`, only visible rows shaped. Split aligns by hunk; a line with no counterpart
faces an empty row. Word diff only on one-for-one pairs, the rule in `types`.

| Controller | Does |
|---|---|
| `workspace.refresh` | reload status, summary, MR, CI and threads for the selected worktree |
| `workspace.set_mode` | base, working, vs-remote; reload the summary |
| `workspace.set_view` | unified or split; remembered per user |
| `workspace.open_file_diff` | load a file's hunks |
| `workspace.expand` | load a slice of the new side |
| `workspace.blame` | load blame for the file |
| `workspace.open_commit` | load one commit's diff |

The sidebar's files tab reads the summary; its tree or list form is a right-click
option.

## Annotations

### Today

`annotation_store`: create, update, resolve, delete, list; a row is session, repo,
path, a line range on the new side, content, author — user or agent — status. The
user creates one by selecting a range; the agent through MCP. *Post to MR* is
`post_mr_comment` with the file and line, which resolves the local copy. MR threads
come from `get_mr_threads`, shown in the Notes tab with their position; `reply_to_thread`
and `resolve_mr_thread` act on the forge. `fix-notes` reads both.

### Planned

**Module `annotations`**, on `db` and `forge`: the store and the post. **Service
`workspace`** holds the notes of the selected worktree — annotations and threads —
as one list.

**One inline layer.** Annotations and threads render at their line in the same block,
author and origin shown, at the code's line height with 6px above and below, full
width. The sidebar's notes tab lists them with file and line; a note opens its line.

| Controller | Does |
|---|---|
| `workspace.create_note` · `workspace.update_note` · `workspace.resolve_note` · `workspace.delete_note` | the store; from the agent, local writes |
| `workspace.post_note` | post as an MR discussion at its line, resolve the local copy |
| `workspace.reply_thread` · `workspace.resolve_thread` | on the forge |

## Editor

### Today

`editor_host`: list files, read, open, save, create file and directory, rename, copy,
delete, search — every path resolved against the worktree root and refused when it
escapes. Open-file state per session. CodeMirror with syntax by extension, search in
file, grep across the worktree with match highlighting, a file tree.

### Planned

**Module `editor`**: the file operations as today. **Module `text`**: rope,
tree-sitter, transactions, the semantic hook. **Service `workspace`** holds the open
file — path, buffer, dirty, cursor.

The editor tab opens from a diff line and returns to the diff on save. A save, a
create, a rename or a delete reaches the diff and the explorer through `watch` like
any other change on disk. The sidebar's explorer tab: file tree, search, grep results. LSP later through the semantic hook:
hover, definition, references, diagnostics.

| Controller | Does |
|---|---|
| `workspace.open_file` | from a diff line or the explorer |
| `workspace.save_file` | write, refresh the diff, return to it |
| `workspace.create_path` · `workspace.rename_path` · `workspace.copy_path` · `workspace.delete_path` | as today |
| `workspace.search` · `workspace.grep` | the sidebar's search bar; `/` selects grep |

## Terminal

### Today

`core/pty`: spawn a child on a pseudo-terminal, stream output in batches — one emit
per window, the first chunk after idle sent at once — accept writes and resizes, end
it. `TERM` and `COLORTERM` set for xterm.js; the child severed from an inherited
tmux or screen. `start_terminal_session` opens a shell with a worktree or the root as
cwd. Base64 both ways.

### Planned

**Base `exec::pty`**: spawn, stream, write, resize, end — the batching kept. **Module
`terminal`**: the `alacritty_terminal` grid over it, dirty flag per frame, resize
through `OnResize`, the list of terminals per worktree. The manual section draws the
grids on `gfx`, split and resizable, collapsible.

| Controller | Does |
|---|---|
| `workspace.open_terminal` | a shell in the selected worktree |
| `workspace.close_terminal` | end it |
| `workspace.split_terminal` | a second one beside the first |

Keys go to the PTY as bytes; nothing is parsed.

## Git

### Today

`worktrees/ops.rs` over `core/git/run.rs`: commit — the agent commits its index, the
UI commits everything — stage and unstage a file or all, push, pull, rebase on main
with continue and abort, discard a file or all. `rebase_conflict` carries the
conflicted files. `worktrees/status.rs`: modified, staged, ahead, behind per
worktree. Ref-moving ops flush the cache where they run.

### Planned

**Base `exec::run`**: the process runner with capture, timeout, kill on drop,
redaction. **Module `git`**: `LC_ALL=C`, `GIT_TERMINAL_PROMPT=0`, the SSH batch flag,
the parsers, the actions. **Service `workspace`** holds the status of the selected
worktree and the conflict list.

The commit box in the sidebar commits the index; stage and unstage per file are UI
actions on functions that exist. The commit box's actions menu: push, pull, rebase,
discard all. A rebase that stops on conflicts reports the files and offers continue
and abort, as today. Not in the first cut: conflict resolution in the workspace.

| Controller | Does |
|---|---|
| `workspace.stage` · `workspace.unstage` | a file, or all |
| `workspace.commit` | the index, with the message |
| `workspace.push` · `workspace.pull` · `workspace.rebase` · `workspace.rebase_continue` · `workspace.rebase_abort` | as today |
| `workspace.discard` · `workspace.discard_all` | as today |

From the agent, every one of these goes through `approvals` unless auto-approve is on.

## Forge

### Today

Direct HTTP to GitLab and GitHub on the shared client; tokens from `gh` and `glab`,
cached, never stored; the forge decided by the remote's host; one trait, two
implementations. One MR per worktree at most: worktree, platform, remote id, URL,
state. Create with the worktree's branch as source and its `base_ref` or the repo
default as target, a footer linking the task; update re-appends the footer; close.
Approve, reply, resolve, comment. Details with approval folded in, CI status and URL,
threads, the review queue across every host in the pool. Nothing polls: CI and threads
move on a push, an MR op or the refresh button.

### Planned

**Module `forge`**, on `http` and `exec`: as today, plus request reviewers and a
review with a verdict — approve, request changes, comment — on both forges, and the
reviewer state in the details. **Service `workspace`** holds the selected worktree's
MR, CI and threads; the `session` slice sums them into `WorktreeDelivery` per worktree.

**Polling**, spawned by the service: only sessions with an open MR, only while the
window is focused, once on focus, fixed interval, one call per MR with threads on the
same tick; a failed poll ages the row to *stale*. `workspace.refresh` fetches the same
at once, by hand.

| Controller | Does |
|---|---|
| `workspace.create_mr` · `workspace.update_mr` · `workspace.close_mr` | through `approvals` from the agent |
| `workspace.request_review` | add reviewers |
| `workspace.review` | post a verdict with the pending annotations as its comments |
| `workspace.comment` | a general note |

The MR surface is one for own and reviewed MRs; CI shows on the worktree row and on
the header for the selected worktree.

## Needs

- [ ] Tree-sitter grammars bundled: yaml, python, bash, json, markdown, dockerfile, go.
- [ ] Word-diff rule in `types`: one-for-one pairs only.
- [ ] Request reviewers and a review with a verdict, both forges; reviewer state in `MrDetails`.
- [ ] The poll's interval and the stale threshold in Config › Preferences.
- [ ] The watcher's debounce window; `notify` as the module's dependency.
