# Workspace — implementation

How the tool sets in [../capabilities/workspace.md](../capabilities/workspace.md) are made,
in the words of [../architecture.md](../architecture.md). Everything here follows the
session's selected worktree. Each table lists the actions still to build; for the rest the
`Command` enum is the truth.

## Diff

**Two documents and an alignment.** A diff is the file before and the file after, each a
document in `text`, and the alignment between them, computed with `imara-diff`. `git` gives
the content at a ref; the working copy comes from disk. Git's own diff output is never
read: no patch parser. Expansion is a slice of a document already held.

**The tree is whole-file, the query is windowed.** A document holds no colours; the frame
asks the tree what the rows on screen mean. A node that opens above the window still
colours the rows inside it.

**Word diff** runs on one-for-one pairs, the rule in `types`. The columns a row of a pair
does not share with the other are marked over the row's ground.

**Two modes:** base, the merge-base with the base branch, the MR's target for a review;
working, uncommitted against HEAD. Untracked files are all additions. A throttled
background fetch keeps the base fresh.

**The whole change is one surface.** The numeric summary builds the file list with no
content. Rows are aligned per file; the files on screen hold their documents, the others
give theirs up. A file's rows fold under its own head row.

**Capped by size.** Above one size a file is aligned but not coloured; above a larger one
it is listed as changed and not shown. A file git calls binary is listed only.

**Module `watch`**: a filesystem watcher on the selected worktree, `.git` excluded,
debounced. A change to a file — by the agent, the editor or a terminal — arrives as
`FilesChanged` and reloads the summary and the changed files under a new generation.

**Reading the worktree never writes it.** Every git command runs with
`GIT_OPTIONAL_LOCKS=0`. Of git's own directory only `HEAD`, `index` and the refs are
reported; the rest is working noise.

**The view — file, inline or split — is the ui's own state**, not a command, and so are the
folds, the find session and the scroll.

**Finding your way** is drawn as [../design.md](../design.md) describes it. A file marked
read dims in the change column; the marks sit on the session, beside the selected worktree.

**Commits.** `get_commits` reads the branch's own log, the base's commits marked as its.
`open_commit` shows one as the change it made, read from the commit and its parent rather
than the disk. A shown commit is read-only: `dispatch` drops every command whose `writes()`
says so, and `leave_commit` puts the working tree back. The sidebar's commits list shows
them; a bar in the commit box's place names the one that stands.

| Still to build | Does |
|---|---|
| `workspace.get_diff` · `workspace.get_status` | reads for the MCP tools |
| `workspace.refresh` | reload status, summary, MR, CI and threads at once |
| `workspace.expand` | a gap's hidden lines, from the document already held |
| `workspace.blame` | blame for the file, uncommitted lines marked |

## Annotations

**Module `annotations`**, on `db` and `forge`: the store and the post. **Service
`workspace`** holds the notes of the selected worktree — annotations and threads — as one
list. A row is session, repo, path, a line range on the new side, content, author, status.

**One inline layer.** A note stands in rows of the surface under the line it was left on:
its mark where that row's number would be, its words on the code's own column, the lines it
covers and its author at the row's end, and a row of buttons under it. A line carrying one
takes its own ground, and one line holds one open note of this session. The sidebar's notes
list shows them with file and line; a note opens its line.

**What a note offers**, by what holds it: its own are written again, resolved, deleted, and
posted where the worktree has an MR; a thread of the forge is replied to and resolved.
`post_note` resolves the local copy once it is up. A verdict carries every open note of the
repo: one call on GitHub, which is atomic, and a note at a time on GitLab, each resolved as
it lands.

## Editor

**Module `editor`**: what the surface needs outside the buffer — the file operations, each
path resolved against the worktree root and refused when it escapes, and the clipboard,
which falls back to one of its own when there is no desktop. **Module `text`**: rope,
tree-sitter, transactions, the semantic hook.

There is no editor tab: the `file` tab holds all three modes, and `editor` is the one that
draws the file itself. It opens any file on that surface — from a diff line, the path term
or the explorer — and stays where it is on save. A save, a create, a rename or a delete
reaches the diff through `watch`, and drops the walk so the explorer reads the worktree
again.

**The sidebar lists one of two things**, picked by the heading: the files that changed, or
the whole worktree as a tree whose open directories the ui remembers. A path typed in the
bar flattens either one to its matches. One `workspace.path` command carries all four
operations; the tree asks for a name in the row the name will stand in, and asks before it
deletes, in that row's own place.

**The rules of editing:**

- **The buffer answers at once, what it implies follows.** A keystroke costs the rope edit
  and a shift of the tree's nodes. The parse and the alignment run in a job and install
  only if the buffer has not moved on. One read is out at a time; a read that lands stale
  starts the next one.
- **A dirty buffer outranks the disk.** While the buffer owes the disk, a write under the
  worktree refreshes the summary and leaves the buffer alone. A save clears the debt.
- **The caret is a place in the document**, carried across a reopen and a change of view,
  clamped to a place that exists.
- **A read that finds the text the buffer holds leaves the buffer alone**, so a save costs
  neither the undo history nor the caret.
- **An undo takes back a typing run**, not a character. A motion or a newline closes the
  run.

**Search** is three things. Module `grep` walks the worktree in parallel for a text,
reports in batches, caps what it returns and stops when the next search starts; the same
module lists the worktree's own paths, which `workspace.list_paths` reads once per search
so the path term narrows the whole worktree and not only the diff. In the surface, a find
session over the open file or the whole change is the ui's own state.

| Still to build | Does |
|---|---|
| `workspace.get_open_file` · `workspace.list_files` · `workspace.read_file` | reads for the MCP tools |

## Terminal

**Base `exec::pty`**: spawn, stream, write, resize, end, batched. **Module `terminal`**:
the `alacritty_terminal` grid over it, a dirty flag per frame, resize through `OnResize`,
the terminals per worktree. The manual section draws the grids, split and resizable.

Keys go to the pty as bytes; nothing is parsed.

| Still to build | Does |
|---|---|
| `workspace.open_terminal` | a shell in the selected worktree |
| `workspace.resize_terminal` | from the pane's layout, to the pty |
| `workspace.close_terminal` · `workspace.split_terminal` | end it; a second one beside the first |

## Git

**Base `exec::run`**: the process runner with capture, timeout, kill on drop, redaction.
**Module `git`**: `LC_ALL=C`, `GIT_TERMINAL_PROMPT=0`, the SSH batch flag, the parsers, the
actions. **Service `workspace`** holds the status of the selected worktree and the conflict
list.

The commit box commits the index; stage and unstage are per file. Its actions menu: push,
pull, discard all. A rebase comes back with conflict resolution, since one without the
other leaves the worktree in a state the app cannot show.

From the agent, every one of these goes through `approvals` unless auto-approve is on.

| Still to build | Does |
|---|---|
| `workspace.rebase` · `workspace.rebase_continue` · `workspace.rebase_abort` | with conflict resolution, not before |
| `workspace.stage_all` · `workspace.unstage_all` | the whole set, from the box |

## Forge

**Module `forge`**: gitlab and github behind one `Remote` enum, the forge decided by the
host alone. Tokens come from module `token` — `gh auth token` and `glab auth status`, held
for the run, never stored. One read call brings a `Snapshot`: the MR, its CI and its
threads. One MR per worktree at most. Create with the worktree's branch as source and its
`base_ref` or the repo default as target, a footer linking the task. Plus requested
reviewers and a review with a verdict on both forges.

The GraphQL transport is `http::Graphql`, shared with `provider`: it carries the token and
turns the errors a forge answers 200 with into an error. GitHub addresses a write by the
node id a read carries; GitLab addresses one by project path and iid, and answers a refused
write in the mutation's own `errors`, which the client reads as the failure.

**Service `workspace`** holds the selected worktree's MR, CI and threads; the `session`
slice sums them into `WorktreeDelivery` per worktree.

**Polling**, on the window's own clock: only while the window is focused, and only the
selected worktree until its forge has been asked once, then every worktree whose row says
its MR is open. One call per MR, with its CI and its threads on the same tick. A read that
fails ages the row to *stale* and leaves what stands. A focus gain asks about everything
again; a push forgets what was asked of that worktree. A host whose forge Groove cannot
read yet is skipped, not reported.

Every read records the `MrFacts` of its worktree. The `task` controller folds them per
task, through the session that works it: several worktrees of one task read as one — the
earliest wait, the worst run, and an approval only where all of them have it.

The writes go through the commit box: `create_mr` is what the one button offers once the
branch is pushed and has no MR, and `update_mr` and `close_mr` hang off its caret while it
has one. The message titles the MR and its remaining lines are the body; an empty box takes
the task's own title, and a footer names the task by url. From the agent they will go
through `approvals`, which slice 6 builds.

**The verdict** is `approve` or `request changes` from the worktree's own menu, in a review
session only, and the commit box's words go up as the comment beside it. GitLab keeps
approval out of GraphQL, so `http` carries the plain calls that stand beside an endpoint.

| Still to build | Does |
|---|---|
| every forge write from the agent | through `approvals`, once it exists |
| `workspace.request_review` | add reviewers |

The MR surface is one for own and reviewed MRs; CI shows on the worktree row and on the
header for the selected worktree.

## Needs

- [x] Tree-sitter grammars bundled: rust, yaml, bash, markdown, python, go. Anything else
      is a plain document.
- [x] A rope: `ropey`. The diff algorithm: upstream `imara-diff`.
- [x] The two size caps: coloured to 1 MiB, aligned and shown to 2 MiB.
- [x] `DiffView` is `File`, `Inline`, `Split`, switched from the file header.
- [x] The word-diff rule in `types`: one-for-one pairs only.
- [x] A file is marked read by the user, never by scrolling past it.
- [ ] Request reviewers and a review with a verdict, both forges; reviewer state in
      `MrDetails`.
- [ ] The poll reads `poll_interval_secs` and `stale_after_secs`, which the config
      already carries and nothing reads; the controller holds its own constant. Slice 7.
- [x] GitLab behind the same `Remote` enum: its own queries, the `glab` token, the `!`
      sigil.
- [ ] A GitLab verdict has no time of its own in the schema, so the MR's own `updatedAt`
      stands for it; the four MR rules age from that on GitLab.
- [ ] The watcher's debounce window.
