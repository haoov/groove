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

**Two documents and an alignment.** A diff is the file before and the file after, each
a document in `text`, and the alignment between them. `git` gives the content at a ref;
the working copy comes from disk. `diff` computes the alignment with `imara-diff`. Git's
own diff output is never read, so there is no patch parser and no drift between a patch
and the files beside it. Expansion is a slice of a document already held. Word diff runs
on one-for-one pairs.

Tree-sitter forces this shape: a hunk parses as garbage on its own, because it starts
mid-expression. Only whole files colour correctly, and search wants the same positions.
The tree is whole-file, the query is not: a document holds no colours, and the frame
asks the tree what the rows on screen mean. 0.3ms for 40 rows, whatever the file holds,
against 20ms for a whole 3000-line file and 227ms for 30000. A node that opens above
the window still colours the rows inside it, because the query takes every node its
range touches.

**Module `diff`**, on `git` and `text`: the three modes, the summary, the alignment,
untracked handling, commits, blame, expansion, the fetch throttle. The cache goes; one
state is refetched on refresh with a generation.

**Service `workspace`** holds per selected worktree: the summary, the documents and
alignments loaded so far, the view — file, inline or split — the mode, the files marked
read, blame when asked.

**Loaded by window.** The numeric summary builds the file list with no content at all.
One file is open at a time today. When the scroll runs across files, documents are held
for the files near the viewport and dropped when they leave it.

**Capped by size.** Above one size a file is aligned but not coloured; above a larger
one it is listed as changed and not shown. A file git calls binary is listed only.

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
faces an empty row. Word diff only on one-for-one pairs, the rule in `types`. `text`
returns the capture names tree-sitter gives, never colours; the capture to role table
lives with the styles.

**Finding your way** is drawn as [../design.md](../design.md) describes it: the common
root stripped once and single-child chains collapsed in the file list, the file and its
syntactic scope pinned at the top, a band where a scroll crosses a directory, and one
column holding the whole change with the viewport as a lens. A file marked read dims in
that column; the marks sit on the session, beside the selected worktree.

| Controller | Does |
|---|---|
| `workspace.get_diff` · `workspace.get_commits` · `workspace.get_status` | reads: the summary, the commit log, git status |
| `workspace.refresh` | reload status, summary, MR, CI and threads for the selected worktree |
| `workspace.set_mode` | base, working, vs-remote; reload the summary |
| `workspace.set_view` | file, inline or split; remembered per user |
| `workspace.mark_read` | a file read or unread, on the session |
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
| `workspace.get_notes` | read: annotations and threads of the selected worktree |
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

**Module `editor`**: what the surface needs outside the buffer — the file operations
as today, each path resolved against the worktree root and refused when it escapes,
and the clipboard, which falls back to one of its own when there is no desktop to
open. **Module `text`**: rope,
tree-sitter, transactions, the semantic hook. **Service `workspace`** holds the open
file — path, buffer, dirty, cursor.

The editor tab opens any file on the same surface, from a diff line or the explorer,
and stays where it is on save: a tab that jumps away under the user is worse than a
tab they leave when they choose. A save, a create, a rename or a delete reaches the
diff and the explorer through `watch` like any other change on disk, and the alignment
follows. The sidebar's explorer tab: file tree, search, grep results. LSP later through the semantic hook:
hover, definition, references, diagnostics.

**Reading the worktree never writes it.** Every git command runs with
`GIT_OPTIONAL_LOCKS=0`, or `git status` rewrites the index on each read and the
watcher on git's own directory wakes itself forever. Of that directory only `HEAD`,
`index` and the refs are reported; the locks, the loose objects and the logs are
git's working noise.

**Editing.** The buffer is the new side of the open file, so every view edits the same
text. Four rules, measured rather than assumed:

- **The buffer answers at once, what it implies follows.** A keystroke costs the rope
  edit and a shift of the tree's nodes: 4µs, whatever the file holds. The parse that
  makes the tree right again costs 1.8ms at 3000 lines from the tree it already has,
  and the alignment as much, so both run in a job and install only if the buffer has
  not moved on. One read is out at a time; a read that lands stale starts the next one.
  The job earns its keep on one text in a hundred: a letter typed before a top-level
  `fn` costs the Rust grammar 193ms at 3000 lines, and a cold parse of that same text
  costs the same, so nothing but a job can hide it.
- **A dirty buffer outranks the disk.** While the buffer owes the disk, a write under
  the worktree refreshes the summary and leaves the buffer alone. A save clears the
  debt, and the reopen that follows lands.
- **The caret is a place in the document**, carried across a reopen and across a
  change of view, clamped to a place that exists. A removed line belongs to the old
  document and takes no caret.
- **An undo takes back a typing run**, not a character. A motion or a newline closes
  the run.

| Controller | Does |
|---|---|
| `workspace.get_open_file` · `workspace.list_files` · `workspace.read_file` | reads |
| `workspace.open_file` | from a diff line or the explorer |
| `workspace.edit` | one keystroke on the buffer: a motion, a change, an undo |
| `workspace.copy` · `workspace.cut` · `workspace.paste` | what the carets hold, through the desktop's clipboard |
| `workspace.stage` · `workspace.unstage` · `workspace.discard` | one path, never more than the row it came from |
| `workspace.message` · `workspace.commit` | the box's own buffer, and the index it spends |
| `workspace.push` · `workspace.pull` | the box's actions menu; HEAD may move, so all of it is read again |
| `workspace.discard_all` | every changed path, asked for in the box first |
| `workspace.save_file` | write; the buffer keeps its place and its history |
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
| `workspace.resize_terminal` | from the pane's layout, to the PTY |
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
actions on functions that exist. The commit box's actions menu: push, pull, discard
all. A rebase comes back with conflict resolution, since one without the other leaves
the worktree in a state the app cannot show.

| Controller | Does |
|---|---|
| `workspace.stage` · `workspace.unstage` | a file, or all |
| `workspace.commit` | the index, with the message |
| `workspace.push` · `workspace.pull` | as today |
| `workspace.rebase` · `workspace.rebase_continue` · `workspace.rebase_abort` | with conflict resolution, not before |
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
| `workspace.get_mr` · `workspace.get_ci` · `workspace.get_threads` · `workspace.get_review_queue` | reads |
| `workspace.create_mr` · `workspace.update_mr` · `workspace.close_mr` | through `approvals` from the agent |
| `workspace.request_review` | add reviewers |
| `workspace.review` | post a verdict with the pending annotations as its comments |
| `workspace.comment` | a general note |

The MR surface is one for own and reviewed MRs; CI shows on the worktree row and on
the header for the selected worktree.

## Needs

- [x] Tree-sitter grammars bundled: rust, yaml, bash, markdown, python, go. Anything
      else is a plain document; toml, json and dockerfile were left for later.
- [x] A rope: `ropey`.
- [x] The diff algorithm: upstream `imara-diff`, not gitoxide's copy. The copy is
      adapted to gitoxide's byte strings and rides its release train; upstream is what
      Helix computes its diff gutter with, and a line diff is a settled algorithm. The
      copy stays the escape hatch, drop-in because it is the same API.
- [x] The two size caps: coloured to 1 MiB, aligned and shown to 2 MiB.
- [ ] Does scrolling past a file mark it read, or only the user?
- [x] `DiffView` is `File`, `Inline`, `Split`, switched from the file header.
- [ ] Word-diff rule in `types`: one-for-one pairs only.
- [ ] Request reviewers and a review with a verdict, both forges; reviewer state in `MrDetails`.
- [ ] The poll's interval and the stale threshold in Config › Preferences.
- [ ] The watcher's debounce window; `notify` as the module's dependency.
